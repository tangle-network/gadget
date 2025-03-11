//! EVM Consumer(s)

use alloc::collections::VecDeque;
use core::pin::Pin;
use core::task::{Context, Poll};

use alloy_provider::fillers::{FillProvider, JoinFill, RecommendedFillers, WalletFiller};
use alloy_provider::network::{Ethereum, EthereumWallet, NetworkWallet, ReceiptResponse};
use alloy_provider::{Network, Provider, RootProvider};
use alloy_rpc_types::TransactionRequest;
use alloy_transport::TransportError;
use blueprint_core::JobResult;
use blueprint_core::error::BoxError;
use futures::Sink;

/// A type alias for the recommended fillers of a network.
pub type RecommendedFillersOf<T> = <T as RecommendedFillers>::RecommendedFillers;

/// A type alias for the Alloy provider with wallet.
pub type AlloyProviderWithWallet<N, W> =
    FillProvider<JoinFill<RecommendedFillersOf<N>, WalletFiller<W>>, RootProvider<N>, N>;

enum State {
    WaitingForResult,
    ProcessingTransaction(Pin<Box<dyn Future<Output = Result<(), TransportError>> + Send>>),
}

impl core::fmt::Debug for State {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::WaitingForResult => write!(f, "WaitingForResult"),
            Self::ProcessingTransaction(_) => f.debug_tuple("ProcessingTransaction").finish(),
        }
    }
}

impl State {
    fn is_waiting(&self) -> bool {
        matches!(self, State::WaitingForResult)
    }
}

/// EVM Consumer for submitting transactions to the EVM
#[derive(Debug)]
pub struct EVMConsumer<W = EthereumWallet, N = Ethereum>
where
    W: NetworkWallet<N> + Clone,
    N: Network + RecommendedFillers,
{
    provider: AlloyProviderWithWallet<N, W>,
    buffer: VecDeque<TransactionRequest>,
    state: State,
}

impl<W, N> EVMConsumer<W, N>
where
    N: Network + RecommendedFillers,
    W: NetworkWallet<N> + Clone,
{
    /// Create a new [`EVMConsumer`]
    pub fn new<P: Provider<N>>(provider: P, wallet: W) -> Self {
        let p = FillProvider::new(provider.root().clone(), N::recommended_fillers())
            .join_with(WalletFiller::new(wallet));
        Self {
            provider: p,
            buffer: VecDeque::new(),
            state: State::WaitingForResult,
        }
    }
}

impl<W, N> Sink<JobResult> for EVMConsumer<W, N>
where
    N: RecommendedFillers + Unpin + 'static,
    N::TransactionRequest: From<TransactionRequest>,
    <N as RecommendedFillers>::RecommendedFillers: Unpin + 'static,
    W: NetworkWallet<N> + Clone + Unpin + 'static,
{
    type Error = BoxError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: JobResult) -> Result<(), Self::Error> {
        blueprint_core::trace!(?item, "Got JobResult");
        let (_, tx_bytes) = item.into_parts()?;
        let tx_request: TransactionRequest = serde_json::from_slice(&tx_bytes)?;
        blueprint_core::trace!(?tx_request, "Got transaction request");
        self.get_mut().buffer.push_back(tx_request);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.buffer.is_empty() && self.state.is_waiting() {
            return Poll::Ready(Ok(()));
        }

        let this = self.get_mut();

        loop {
            match this.state {
                State::WaitingForResult => {
                    let Some(tx) = this.buffer.pop_front() else {
                        return Poll::Ready(Ok(()));
                    };
                    let fut = send_transaction(this.provider.clone(), tx);
                    this.state = State::ProcessingTransaction(Box::pin(fut));
                }
                State::ProcessingTransaction(ref mut fut) => match fut.as_mut().poll(cx) {
                    Poll::Ready(Ok(())) => {
                        this.state = State::WaitingForResult;
                    }
                    Poll::Ready(Err(err)) => {
                        return Poll::Ready(Err(err.into()));
                    }
                    Poll::Pending => return Poll::Pending,
                },
            }
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.buffer.is_empty() {
            Poll::Ready(Ok(()))
        } else {
            self.poll_flush(cx)
        }
    }
}

async fn send_transaction<N, P>(provider: P, tx: TransactionRequest) -> Result<(), TransportError>
where
    N: Network,
    N::TransactionRequest: From<TransactionRequest>,
    P: Provider<N>,
{
    use alloy_transport::TransportErrorKind;
    let res = provider.send_transaction(tx.into()).await;
    let receipt_res = match res {
        Ok(pending_tx) => pending_tx.get_receipt().await,
        Err(err) => {
            blueprint_core::error!("Failed to send transaction: {:?}", err);
            return Err(err);
        }
    };

    let receipt = match receipt_res {
        Ok(receipt) => receipt,
        Err(err) => {
            blueprint_core::error!("Pending transaction failed: {:?}", err);
            return Err(TransportError::Transport(TransportErrorKind::Custom(
                Box::new(err),
            )));
        }
    };

    blueprint_core::debug!(
        status = %receipt.status(),
        block_hash = ?receipt.block_hash(),
        block_number = ?receipt.block_number(),
        transaction_hash = %receipt.transaction_hash(),
        transaction_index = ?receipt.transaction_index(),
        gas_used = %receipt.gas_used(),
        effective_gas_price = %receipt.effective_gas_price(),
        "Transaction sent successfully",
    );

    Ok(())
}
