use crate::network::ZkNetworkService;
use crate::protocol::ZkProtocol;
use async_trait::async_trait;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient, PalletSubmitter};
use gadget_common::config::NetworkAndProtocolSetup;
use gadget_common::debug_logger::DebugLogger;
use gadget_common::Error;
use mpc_net::prod::RustlsCertificate;
use pallet_jobs_rpc_runtime_api::JobsApi;
use protocol_macros::protocol;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_rustls::rustls::{Certificate, PrivateKey, RootCertStore};

pub mod network;
pub mod protocol;

#[protocol]
pub struct ZkGadgetConfig<B: Block, C: ClientWithApi<B, BE>, BE: Backend<B>>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pub king_bind_addr: Option<SocketAddr>,
    pub client_only_king_addr: Option<SocketAddr>,
    pub public_identity_der: Vec<u8>,
    pub private_identity_der: Vec<u8>,
    pub client_only_king_public_identity_der: Option<Vec<u8>>,
    pub account_id: AccountId,
    pub pallet_tx: Arc<dyn PalletSubmitter>,
    pub client: C,
    pub logger: DebugLogger,
    pub _pd: std::marker::PhantomData<(B, C, BE)>,
}

#[async_trait]
impl<B: Block, C: ClientWithApi<B, BE>, BE: Backend<B>> NetworkAndProtocolSetup
    for ZkGadgetConfig<B, C, BE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type Network = ZkNetworkService;
    type Protocol = ZkProtocol<B, C, BE>;
    type Client = C;
    type Block = B;
    type Backend = BE;

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Block, Self::Backend, Self::Client>,
    ) -> Result<(Self::Network, Self::Protocol), Error> {
        let network = create_zk_network(self).await?;
        let zk_protocol = ZkProtocol {
            client: jobs_client,
            account_id: self.account_id,
            network: network.clone(),
            logger: self.logger.clone(),
        };

        Ok((network, zk_protocol))
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

pub async fn create_zk_network<B: Block, C: ClientWithApi<B, BE>, BE: Backend<B>>(
    config: &ZkGadgetConfig<B, C, BE>,
) -> Result<ZkNetworkService, Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    let our_identity = RustlsCertificate {
        cert: Certificate(config.public_identity_der.clone()),
        private_key: PrivateKey(config.private_identity_der.clone()),
    };

    if let Some(addr) = &config.king_bind_addr {
        ZkNetworkService::new_king(config.account_id, *addr, our_identity).await
    } else {
        let king_addr = config
            .client_only_king_addr
            .expect("King address must be specified if king bind address is not specified");

        let mut king_certs = RootCertStore::empty();
        king_certs
            .add(&Certificate(
                config
                    .client_only_king_public_identity_der
                    .clone()
                    .expect("The client must specify the identity of the king"),
            ))
            .map_err(|err| Error::InitError {
                err: err.to_string(),
            })?;

        ZkNetworkService::new_client(king_addr, config.account_id, our_identity, king_certs).await
    }
}
