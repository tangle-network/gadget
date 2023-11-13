use crate::module::ZkModule;
use crate::network::{RegistantId, ZkNetworkService};
use gadget_core::gadget::substrate::Client;
use gadget_core::{Backend, Block};
use mpc_net::prod::RustlsCertificate;
use std::net::SocketAddr;
use tokio_rustls::rustls::{Certificate, PrivateKey, RootCertStore};
use webb_gadget::Error;

pub mod module;
pub mod network;

pub struct ZkGadgetConfig {
    king_bind_addr: Option<SocketAddr>,
    client_only_king_addr: Option<SocketAddr>,
    id: RegistantId,
    public_identity_der: Vec<u8>,
    private_identity_der: Vec<u8>,
    client_only_king_public_identity_der: Option<Vec<u8>>,
}

pub async fn run<C: Client<B, BE>, B: Block, BE: Backend<B>>(
    config: ZkGadgetConfig,
    client: C,
) -> Result<(), Error> {
    // Create the zk gadget module
    let network = create_zk_network(&config).await?;

    let zk_module = ZkModule {}; // TODO: proper implementation

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
