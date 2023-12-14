use crate::client_ext::ClientWithApi;
use crate::network::{RegistantId, ZkNetworkService};
use crate::protocol::proto_gen::{AsyncProtocolGenerator, ZkProtocolGenerator};
use crate::protocol::{AdditionalProtocolParams, ZkProtocol};
use gadget_common::gadget::network::Network;
use gadget_common::Error;
use mpc_net::prod::RustlsCertificate;
use pallet_jobs_rpc_runtime_api::JobsApi;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_core::ecdsa;
use sp_runtime::traits::Block;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_rustls::rustls::{Certificate, PrivateKey, RootCertStore};

pub mod network;
pub mod protocol;

pub mod client_ext;

pub struct ZkGadgetConfig {
    pub king_bind_addr: Option<SocketAddr>,
    pub client_only_king_addr: Option<SocketAddr>,
    pub id: RegistantId,
    pub n_parties: usize,
    pub public_identity_der: Vec<u8>,
    pub private_identity_der: Vec<u8>,
    pub client_only_king_public_identity_der: Option<Vec<u8>>,
}

pub struct ZkSaaSConfig {}

pub async fn run<B, C>(config: ZkSaaSConfig, client: C) -> Result<(), Error>
where
    B: Block,
    C: ClientWithApi<B>,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, ecdsa::Public>,
{
    let client = Arc::new(client);
}

pub async fn run_zk_saas<
    C: ClientWithApi<B>,
    B: Block,
    T: AdditionalProtocolParams,
    Gen: AsyncProtocolGenerator<T, Error, ZkNetworkService, C, B>,
>(
    config: ZkGadgetConfig,
    client: C,
    extra_parameters: T,
    proto_gen: Gen,
) -> Result<(), Error> {
    // Create the zk gadget module
    let network = create_zk_network(&config).await?;
    log::info!("Created zk network for party {}", config.id);

    let protocol = ZkProtocolGenerator {
        party_id: config.id,
        // TODO: The below should be inside job metadata, but NOT determined at runtime
        n_parties: config.n_parties,
        extra_parameters,
        network: network.clone(),
        client: client.clone(),
        proto_gen: Box::new(proto_gen),
    };

    let zk_protocol = ZkProtocol {
        party_id: config.id,
        additional_params: (),
        client: client.clone(),
        protocol,
        _pd: std::marker::PhantomData,
    };

    // Plug the module into the webb gadget
    gadget_common::run_protocol(network, zk_protocol, client).await
}

pub async fn create_zk_network(config: &ZkGadgetConfig) -> Result<ZkNetworkService, Error> {
    let our_identity = RustlsCertificate {
        cert: Certificate(config.public_identity_der.clone()),
        private_key: PrivateKey(config.private_identity_der.clone()),
    };
    if let Some(addr) = &config.king_bind_addr {
        ZkNetworkService::new_king(*addr, our_identity).await
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

        ZkNetworkService::new_client(king_addr, config.id, our_identity, king_certs).await
    }
}
