use crate::client_ext::ClientWithApi;
use crate::module::proto_gen::AsyncProtocolGenerator;
use crate::module::{AdditionalProtocolParams, ZkModule};
use crate::network::{RegistantId, ZkNetworkService};
use gadget_core::Block;
use mpc_net::prod::RustlsCertificate;
use std::net::SocketAddr;
use tokio_rustls::rustls::{Certificate, PrivateKey, RootCertStore};
use webb_gadget::Error;

pub mod module;
pub mod network;

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

pub async fn run<
    C: ClientWithApi<B>,
    B: Block,
    T: AdditionalProtocolParams,
    Gen: AsyncProtocolGenerator<T, Error, ZkNetworkService, C, B>,
>(
    config: ZkGadgetConfig,
    client: C,
    additional_protocol_params: T,
    async_protocol_generator: Gen,
) -> Result<(), Error> {
    // Create the zk gadget module
    let network = create_zk_network(&config).await?;
    log::info!("Created zk network for party {}", config.id);

    let zk_module = ZkModule {
        party_id: config.id,
        n_parties: config.n_parties,
        additional_protocol_params,
        network: network.clone(),
        client: client.clone(),
        async_protocol_generator: Box::new(async_protocol_generator),
    };

    // Plug the module into the webb gadget
    webb_gadget::run(network, zk_module, client).await
}

async fn create_zk_network(config: &ZkGadgetConfig) -> Result<ZkNetworkService, Error> {
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
