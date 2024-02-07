use gadget_common::client::{AccountId, ClientWithApi, JobsClient, PalletSubmitter};
use gadget_common::config::NetworkAndProtocolSetup;
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::network::Network;
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use pallet_jobs_rpc_runtime_api::JobsApi;
use protocol_macros::protocol;
use protocols::key_refresh::DfnsCGGMP21KeyRefreshProtocol;
use protocols::key_rotate::DfnsCGGMP21KeyRotateProtocol;
use protocols::keygen::DfnsCGGMP21KeygenProtocol;
use protocols::sign::DfnsCGGMP21SigningProtocol;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::sync::Arc;

pub mod constants;
pub mod error;
pub mod protocols;
pub mod util;

/// A Helper macro to declare a protocol, used
/// to avoid code duplication.
macro_rules! decl_porto {
    ($name:ident + $proto:ident = $im:path) => {

        #[protocol]
        pub struct $name<
            N: Network,
            B: Block,
            BE: Backend<B>,
            KBE: KeystoreBackend,
            C: ClientWithApi<B, BE>,
        > where
            <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId, MaxParticipants, MaxSubmissionLen, MaxKeyLen, MaxDataLen, MaxSignatureLen, MaxProofLen>,
        {
            pub account_id: AccountId,
            pub network: N,
            pub keystore_backend: ECDSAKeyStore<KBE>,
            pub client: C,
            pub logger: DebugLogger,
            pub pallet_tx: Arc<dyn PalletSubmitter>,
            pub _pd: std::marker::PhantomData<(B, BE)>,
        }

        #[async_trait::async_trait]
        impl<N: Network, B: Block, BE: Backend<B>, KBE: KeystoreBackend, C: ClientWithApi<B, BE>>
            NetworkAndProtocolSetup for $name<N, B, BE, KBE, C>
        where
            <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId, MaxParticipants, MaxSubmissionLen, MaxKeyLen, MaxDataLen, MaxSignatureLen, MaxProofLen>,
        {
            type Network = N;
            type Protocol = $proto<B, BE, KBE, C, N>;
            type Client = C;
            type Block = B;
            type Backend = BE;

            async fn build_network_and_protocol(
                &self,
                jobs_client: JobsClient<Self::Block, Self::Backend, Self::Client>,
            ) -> Result<(Self::Network, Self::Protocol), gadget_common::Error> {
                use $im as m;
                let protocol = m::create_protocol(
                    self.account_id,
                    jobs_client,
                    self.network.clone(),
                    self.logger.clone(),
                    self.keystore_backend.clone(),
                )
                .await;

                Ok((self.network.clone(), protocol))
            }

            fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
                self.pallet_tx.clone()
            }

            fn logger(&self) -> DebugLogger {
                self.logger.clone()
            }

            fn client(&self) -> Self::Client {
                self.client.clone()
            }
        }

    };
    // recursive case with optional trailing comma
    ($($name:ident + $proto:ident = $im:path),+ $(,)?) => {
        $(decl_porto!($name + $proto = $im);)+
    };
}

// A macro to declare all the protocols
decl_porto!(
    DfnsCGGMP21KeygenProtocolConfig + DfnsCGGMP21KeygenProtocol = protocols::keygen,
    DfnsCGGMP21SigningProtocolConfig + DfnsCGGMP21SigningProtocol = protocols::sign,
    DfnsCGGMP21KeyRefreshProtocolConfig + DfnsCGGMP21KeyRefreshProtocol = protocols::key_refresh,
    DfnsCGGMP21KeyRotateProtocolConfig + DfnsCGGMP21KeyRotateProtocol = protocols::key_rotate,
);

#[allow(clippy::too_many_arguments)]
pub async fn run<B, BE, KBE, C, N, Tx>(
    account_id: AccountId,
    logger: DebugLogger,
    keystore: ECDSAKeyStore<KBE>,
    pallet_tx: Tx,
    (client_keygen, client_signing, client_key_refresh, client_key_rotate): (C, C, C, C),
    (network_keygen, network_signing, network_key_refresh, network_key_rotate): (N, N, N, N),
) -> Result<(), gadget_common::Error>
where
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    Tx: PalletSubmitter,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<
        B,
        AccountId,
        MaxParticipants,
        MaxSubmissionLen,
        MaxKeyLen,
        MaxDataLen,
        MaxSignatureLen,
        MaxProofLen,
    >,
{
    let pallet_tx = Arc::new(pallet_tx) as Arc<dyn PalletSubmitter>;
    let keygen_config = DfnsCGGMP21KeygenProtocolConfig {
        account_id,
        network: network_keygen,
        keystore_backend: keystore.clone(),
        client: client_keygen,
        logger: logger.clone(),
        pallet_tx: pallet_tx.clone(),
        _pd: std::marker::PhantomData,
    };

    let sign_config = DfnsCGGMP21SigningProtocolConfig {
        account_id,
        network: network_signing,
        keystore_backend: keystore.clone(),
        client: client_signing,
        logger: logger.clone(),
        pallet_tx: pallet_tx.clone(),
        _pd: std::marker::PhantomData,
    };

    let key_refresh_config = DfnsCGGMP21KeyRefreshProtocolConfig {
        account_id,
        network: network_key_refresh,
        keystore_backend: keystore.clone(),
        client: client_key_refresh,
        logger: logger.clone(),
        pallet_tx: pallet_tx.clone(),
        _pd: std::marker::PhantomData,
    };

    let key_rotate_config = DfnsCGGMP21KeyRotateProtocolConfig {
        account_id,
        network: network_key_rotate,
        keystore_backend: keystore,
        client: client_key_rotate,
        logger,
        pallet_tx,
        _pd: std::marker::PhantomData,
    };

    let keygen_future = keygen_config.execute();
    let sign_future = sign_config.execute();
    let key_refresh_future = key_refresh_config.execute();
    let key_rotate_future = key_rotate_config.execute();

    tokio::select! {
        res0 = keygen_future => res0,
        res1 = sign_future => res1,
        res2 = key_refresh_future => res2,
        res3 = key_rotate_future => res3,
    }
}
