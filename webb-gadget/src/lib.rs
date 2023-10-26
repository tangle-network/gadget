use crate::gadget::registry::RegistantId;
use crate::gadget::ZkGadget;
use gadget_core::gadget::manager::{GadgetError, GadgetManager};
use gadget_core::gadget::substrate::SubstrateGadget;
use mpc_net::prod::RustlsCertificate;
use std::net::SocketAddr;
use tokio_rustls::rustls::{Certificate, PrivateKey, RootCertStore};

pub mod gadget;

#[derive(Debug)]
pub enum Error {
    RegistryCreateError { err: String },
    RegistrySendError { err: String },
    RegistryRecvError { err: String },
    GadgetManagerError { err: GadgetError },
    InitError { err: String },
}

pub struct GadgetConfig {
    king_bind_addr: Option<SocketAddr>,
    client_only_king_addr: Option<SocketAddr>,
    id: RegistantId,
    public_identity_der: Vec<u8>,
    private_identity_der: Vec<u8>,
    client_only_king_public_identity_der: Option<Vec<u8>>,
}

pub async fn run(config: GadgetConfig) -> Result<(), Error> {
    let client = Default::default(); // TODO: Replace with a real substrate client
    let now = 0; // TODO: Replace with a real clock from client

    // Create the zk gadget module
    let our_identity = RustlsCertificate {
        cert: Certificate(config.public_identity_der),
        private_key: PrivateKey(config.private_identity_der),
    };

    // TODO: make registry a gossip networking, since this will be the most generalized for the modularity
    // we are seeking for all possible async protocol types
    let gadget_module = if let Some(addr) = &config.king_bind_addr {
        ZkGadget::new_king(*addr, our_identity, now).await?
    } else {
        let king_addr = config
            .client_only_king_addr
            .expect("King address must be specified if king bind address is not specified");

        let mut king_certs = RootCertStore::empty();
        king_certs
            .add(&Certificate(
                config
                    .client_only_king_public_identity_der
                    .expect("The client must specify the identity of the king"),
            ))
            .map_err(|err| Error::InitError {
                err: err.to_string(),
            })?;

        ZkGadget::new_client(king_addr, config.id, our_identity, king_certs, now).await?
    };

    // Plug the module into the substrate gadget
    let substrate_gadget = SubstrateGadget::new(client, gadget_module);

    // Run the GadgetManager to execute the substrate gadget
    GadgetManager::new(substrate_gadget)
        .await
        .map_err(|err| Error::GadgetManagerError { err })
}
