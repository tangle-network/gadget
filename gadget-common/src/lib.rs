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

pub mod channels;
pub mod client;
pub mod config;
pub mod debug_logger;
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
