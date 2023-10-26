use crate::gadget::message::GadgetProtocolMessage;
use crate::gadget::registry::RegistantId;
use crate::gadget::work_manager::ZKWorkManager;
use crate::Error;
use async_trait::async_trait;
use gadget_core::gadget::substrate::SubstrateGadgetModule;
use gadget_core::job_manager::{PollMethod, ProtocolWorkManager};
use mpc_net::prod::RustlsCertificate;
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_rustls::rustls::RootCertStore;

pub mod registry;
pub mod work_manager;

pub mod async_protocols;

pub mod message;

/// Used as a module to place inside the SubstrateGadget
///
/// The zkGadget will need to create async protocols for each job it receives from the blockchain.
/// When it does so, since the clients may change, we will need to also update the TLS certs of
/// the king to match the new clients. As such, for each new async protocol we spawn, we will
/// also need to create a new [`ProdNet`] instance for the king and the clients
pub struct ZkGadget {
    registry: registry::RegistryService,
    job_manager: ProtocolWorkManager<ZKWorkManager>,
    from_registry: Mutex<tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>>,
    clock: Arc<RwLock<u64>>,
}

const MAX_ACTIVE_TASKS: usize = 4;
const MAX_PENDING_TASKS: usize = 4;

impl ZkGadget {
    pub async fn new_king<T: tokio::net::ToSocketAddrs>(
        bind_addr: SocketAddr,
        identity: RustlsCertificate,
        now: u64,
    ) -> Result<Self, Error> {
        let registry = registry::RegistryService::new_king(bind_addr, identity).await?;
        Ok(Self::new_inner(registry, now))
    }

    pub async fn new_client<T: std::net::ToSocketAddrs>(
        king_registry_addr: T,
        registrant_id: RegistantId,
        client_identity: RustlsCertificate,
        king_cert: RootCertStore,
        now: u64,
    ) -> Result<Self, Error> {
        let registry = registry::RegistryService::new_client(
            king_registry_addr,
            registrant_id,
            client_identity,
            king_cert,
        )
        .await?;
        Ok(Self::new_inner(registry, now))
    }

    fn new_inner(mut registry: registry::RegistryService, now: u64) -> Self {
        let clock = Arc::new(RwLock::new(now));
        let clock_clone = clock.clone();
        let from_registry = registry.take_subscription_channel().expect("Should exist");

        let job_manager_zk = ZKWorkManager {
            clock: Arc::new(move || *clock_clone.read()),
        };

        let job_manager = ProtocolWorkManager::new(
            job_manager_zk,
            MAX_ACTIVE_TASKS,
            MAX_PENDING_TASKS,
            PollMethod::Interval { millis: 200 },
        );

        ZkGadget {
            registry,
            job_manager,
            clock,
            from_registry: Mutex::new(from_registry),
        }
    }
}

#[async_trait]
impl SubstrateGadgetModule for ZkGadget {
    type Error = Error;
    type FinalityNotification = ();
    type BlockImportNotification = ();
    type ProtocolMessage = GadgetProtocolMessage;

    async fn get_next_protocol_message(&self) -> Option<Self::ProtocolMessage> {
        self.from_registry.lock().await.recv().await
    }

    async fn process_finality_notification(
        &self,
        notification: Self::FinalityNotification,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn process_block_import_notification(
        &self,
        notification: Self::BlockImportNotification,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn process_protocol_message(
        &self,
        message: Self::ProtocolMessage,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn process_error(&self, error: Self::Error) {
        todo!()
    }
}
