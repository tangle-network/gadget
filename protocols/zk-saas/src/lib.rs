use crate::network::ZkNetworkService;
use crate::protocol::ZkProtocol;
use gadget_common::client::{create_client, AccountId, ClientWithApi, PalletSubmitter};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::Error;
use mpc_net::prod::RustlsCertificate;
use pallet_jobs_rpc_runtime_api::JobsApi;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_rustls::rustls::{Certificate, PrivateKey, RootCertStore};

pub mod network;
pub mod protocol;

pub struct ZkGadgetConfig {
    pub king_bind_addr: Option<SocketAddr>,
    pub client_only_king_addr: Option<SocketAddr>,
    pub public_identity_der: Vec<u8>,
    pub private_identity_der: Vec<u8>,
    pub client_only_king_public_identity_der: Option<Vec<u8>>,
    pub account_id: AccountId,
}

pub async fn run<
    C: ClientWithApi<B, BE> + 'static,
    B: Block,
    BE: Backend<B> + 'static,
    P: PalletSubmitter,
>(
    config: ZkGadgetConfig,
    logger: DebugLogger,
    client: C,
    pallet_tx: P,
) -> Result<(), Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    // Create the zk gadget module
    let network = create_zk_network(&config).await?;
    logger.info(format!(
        "Created zk network for party {}",
        config.account_id
    ));

    let client_inner = Arc::new(client);
    let client = create_client(client_inner, logger.clone(), Arc::new(pallet_tx)).await?;
    let zk_protocol = ZkProtocol {
        client: client.clone(),
        account_id: config.account_id,
        network: network.clone(),
        logger,
    };

    // Plug the protocol into the webb gadget
    gadget_common::run_protocol(network, zk_protocol, client).await
}

pub async fn create_zk_network(config: &ZkGadgetConfig) -> Result<ZkNetworkService, Error> {
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
