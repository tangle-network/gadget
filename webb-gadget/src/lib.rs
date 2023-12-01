use crate::gadget::network::Network;
use crate::gadget::work_manager::WebbWorkManager;
use crate::gadget::{WebbGadgetProtocol, WebbModule};
use futures_util::future::TryFutureExt;
use gadget_core::gadget::manager::{GadgetError, GadgetManager};
use gadget_core::gadget::substrate::{Client, SubstrateGadget};
use gadget_core::job_manager::{PollMethod, ProtocolWorkManager, WorkManagerError};
use parking_lot::RwLock;
pub use sc_client_api::BlockImportNotification;
pub use sc_client_api::{Backend, FinalityNotification};
pub use sp_runtime::traits::{Block, Header};
use sp_runtime::SaturatedConversion;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

pub mod gadget;

pub mod helpers;
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
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}
pub async fn run_protocol_with_wm<C: Client<B>, B: Block, N: Network, P: WebbGadgetProtocol<B>>(
    network: N,
    protocol: P,
    client: C,
    work_manager: ProtocolWorkManager<WebbWorkManager>,
) -> Result<(), Error> {
    let webb_module = WebbModule::new(network.clone(), protocol, work_manager);
    // Plug the module into the substrate gadget to interface the WebbGadget with Substrate
    let substrate_gadget = SubstrateGadget::new(client, webb_module);

    let network_future = network.run();
    let gadget_future =
        GadgetManager::new(substrate_gadget).map_err(|err| Error::GadgetManagerError { err });

    // Run both the network and the gadget together
    tokio::try_join!(network_future, gadget_future).map(|_| ())
}

pub async fn run_protocol<C: Client<B>, B: Block, N: Network, P: WebbGadgetProtocol<B>>(
    network: N,
    protocol: P,
    client: C,
) -> Result<(), Error> {
    let wm = create_work_manager(&client, &protocol).await?;
    run_protocol_with_wm(network, protocol, client, wm).await
}

pub async fn create_work_manager<C: Client<B>, B: Block, P: WebbGadgetProtocol<B>>(
    client: &C,
    protocol: &P,
) -> Result<ProtocolWorkManager<WebbWorkManager>, Error> {
    // Before running, wait for the first finality notification we receive
    let now: u64 = (*client
        .get_latest_finality_notification()
        .await
        .ok_or_else(|| Error::InitError {
            err: "No finality notification received".to_string(),
        })?
        .header
        .number())
    .saturated_into();

    let work_manager_config = protocol.get_work_manager_config();

    let clock = Arc::new(RwLock::new(Some(now)));

    let job_manager_zk = WebbWorkManager { clock };

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
