use crate::client::{AccountId, ClientWithApi, JobsApiForGadget};
use crate::config::{NetworkAndProtocolSetup, ProtocolConfig};
use crate::gadget::work_manager::WorkManager;
use crate::gadget::{GadgetProtocol, Module};
use gadget::network::Network;
use gadget_core::gadget::manager::{AbstractGadget, GadgetError, GadgetManager};
use gadget_core::gadget::substrate::{Client, SubstrateGadget};
pub use gadget_core::job::JobError;
pub use gadget_core::job::*;
pub use gadget_core::job_manager::WorkManagerInterface;
pub use gadget_core::job_manager::{PollMethod, ProtocolWorkManager, WorkManagerError};
use parking_lot::RwLock;
pub use sc_client_api::BlockImportNotification;
pub use sc_client_api::{Backend, FinalityNotification};
use sp_api::ProvideRuntimeApi;
pub use sp_core;
pub use sp_runtime::traits::{Block, Header};
use sp_runtime::SaturatedConversion;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use tokio::task::JoinError;

pub mod prelude {
    pub use crate::client::*;
    pub use crate::config::*;
    pub use crate::full_protocol::{FullProtocolConfig, NodeInput};
    pub use crate::gadget::message::GadgetProtocolMessage;
    pub use crate::gadget::work_manager::WorkManager;
    pub use crate::gadget::JobInitMetadata;
    pub use crate::generate_setup_and_run_command;
    pub use crate::keystore::{ECDSAKeyStore, InMemoryBackend, KeystoreBackend};
    pub use crate::{BuiltExecutableJobWrapper, Error, JobBuilder, JobError, WorkManagerInterface};
    pub use async_trait::async_trait;
    pub use gadget_core::job_manager::SendFuture;
    pub use parking_lot::Mutex;
    pub use protocol_macros::protocol;
    pub use std::pin::Pin;
    pub use std::sync::Arc;
    pub use tangle_primitives::jobs::*;
    pub use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
    pub use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
}
pub mod channels;
pub mod client;
pub mod config;
pub mod debug_logger;
pub mod full_protocol;
pub mod gadget;
pub mod helpers;
pub mod keystore;
pub mod locks;
pub mod protocol;
#[derive(Debug)]
pub enum Error {
    RegistryCreateError { err: String },
    RegistrySendError { err: String },
    RegistryRecvError { err: String },
    RegistrySerializationError { err: String },
    RegistryListenError { err: String },
    GadgetManagerError { err: GadgetError },
    InitError { err: String },
    WorkManagerError { err: WorkManagerError },
    ProtocolRemoteError { err: String },
    ClientError { err: String },
    JobError { err: JobError },
    NetworkError { err: String },
    KeystoreError { err: String },
    MissingNetworkId,
    PeerNotFound { id: AccountId },
    JoinError { err: JoinError },
    ParticipantNotSelected { id: AccountId, reason: String },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

impl From<JobError> for Error {
    fn from(err: JobError) -> Self {
        Error::JobError { err }
    }
}

pub async fn run_protocol<T: ProtocolConfig>(mut protocol_config: T) -> Result<(), Error>
where
    <<T::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Client as ProvideRuntimeApi<
        <T::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Block,
    >>::Api: JobsApiForGadget<<T::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Block>,
{
    let client = protocol_config.take_client();
    let network = protocol_config.take_network();
    let protocol = protocol_config.take_protocol();

    // Before running, wait for the first finality notification we receive
    let latest_finality_notification =
        get_latest_finality_notification_from_client(&client).await?;
    let work_manager = create_work_manager(&latest_finality_notification, &protocol).await?;
    let proto_module = Module::new(network.clone(), protocol, work_manager);
    // Plug the module into the substrate gadget to interface the WebbGadget with Substrate
    let substrate_gadget = SubstrateGadget::new(client, proto_module);
    let network_future = network.run();
    let gadget_future = async move {
        // Poll the first finality notification to ensure clients can execute without having to wait
        // for another block to be produced
        if let Err(err) = substrate_gadget
            .process_finality_notification(latest_finality_notification)
            .await
        {
            substrate_gadget.process_error(err).await;
        }

        GadgetManager::new(substrate_gadget)
            .await
            .map_err(|err| Error::GadgetManagerError { err })
    };

    // Run both the network and the gadget together
    tokio::try_join!(network_future, gadget_future).map(|_| ())
}

/// Creates a work manager
pub async fn create_work_manager<
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    P: GadgetProtocol<B, BE, C>,
>(
    latest_finality_notification: &FinalityNotification<B>,
    protocol: &P,
) -> Result<ProtocolWorkManager<WorkManager>, Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    let now: u64 = (*latest_finality_notification.header.number()).saturated_into();

    let work_manager_config = protocol.get_work_manager_config();

    let clock = Arc::new(RwLock::new(Some(now)));

    let job_manager_zk = WorkManager {
        clock,
        logger: protocol.logger().clone(),
    };

    let poll_method = match work_manager_config.interval {
        Some(interval) => PollMethod::Interval {
            millis: interval.as_millis() as u64,
        },
        None => PollMethod::Manual,
    };

    Ok(ProtocolWorkManager::new(
        job_manager_zk,
        work_manager_config.max_active_tasks,
        work_manager_config.max_pending_tasks,
        poll_method,
    ))
}

async fn get_latest_finality_notification_from_client<C: Client<B>, B: Block>(
    client: &C,
) -> Result<FinalityNotification<B>, Error> {
    client
        .get_latest_finality_notification()
        .await
        .ok_or_else(|| Error::InitError {
            err: "No finality notification received".to_string(),
        })
}

#[macro_export]
/// Generates a run function that returns a future that runs all the supplied protocols run concurrently
/// Also generates a setup_node function that sets up the future that runs all the protocols concurrently
#[allow(clippy::crate_in_macro_def)]
macro_rules! generate_setup_and_run_command {
    ($( $config:ident ),*) => {
        /// Sets up a future that runs all the protocols concurrently
        pub fn setup_node<B: Block, BE: Backend<B> + 'static, C: ClientWithApi<B, BE>, N: Network, KBE: gadget_common::keystore::KeystoreBackend, D: Send + Clone + 'static>(node_input: NodeInput<B, BE, C, N, KBE, D>) -> impl SendFuture<'static, ()>
                where
            <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,{
            async move {
                if let Err(err) = run(
                    node_input.mock_clients,
                    node_input.pallet_tx,
                    node_input.mock_networks,
                    node_input.logger.clone(),
                    node_input.account_id,
                    node_input.keystore,
                )
                .await
                {
                    node_input
                        .logger
                        .error(format!("Error running gadget: {:?}", err));
                }
            }
        }

        pub async fn run<B: Block, BE: Backend<B> + 'static, C: ClientWithApi<B, BE>, N: Network, KBE: gadget_common::keystore::KeystoreBackend>(
            mut client: Vec<C>,
            pallet_tx: Arc<dyn PalletSubmitter>,
            mut network: Vec<N>,
            logger: DebugLogger,
            account_id: AccountId,
            key_store: ECDSAKeyStore<KBE>,
        ) -> Result<(), Error>
        where
            <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
        {
            use futures::TryStreamExt;
            let futures = futures::stream::FuturesUnordered::new();

            $(
                let config = crate::$config::new(client.pop().expect("Not enough clients"), pallet_tx.clone(), network.pop().expect("Not enough networks"), logger.clone(), account_id.clone(), key_store.clone()).await?;
                futures.push(Box::pin(config.execute()) as std::pin::Pin<Box<dyn SendFuture<'static, Result<(), gadget_common::Error>>>>);
            )*

            futures.try_collect::<Vec<_>>().await.map(|_| ())
        }
    };
}
