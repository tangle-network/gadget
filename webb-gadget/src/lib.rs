use crate::gadget::network::Network;
use crate::gadget::{WebbGadgetModule, WebbModule};
use futures_util::future::TryFutureExt;
use gadget_core::gadget::manager::{GadgetError, GadgetManager};
use gadget_core::gadget::substrate::{Client, SubstrateGadget};
use gadget_core::job_manager::WorkManagerError;
pub use sc_client_api::BlockImportNotification;
pub use sc_client_api::{Backend, FinalityNotification};
use sp_runtime::traits::{Block, Header};
use sp_runtime::SaturatedConversion;
use std::fmt::{Debug, Display, Formatter};

pub mod gadget;

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

pub async fn run<C: Client<B>, B: Block, N: Network, M: WebbGadgetModule<B>>(
    network: N,
    module: M,
    client: C,
) -> Result<(), Error> {
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
    let webb_module = WebbModule::new(network.clone(), module, Some(now));
    // Plug the module into the substrate gadget to interface the WebbGadget with Substrate
    let substrate_gadget = SubstrateGadget::new(client, webb_module);

    let network_future = network.run();
    let gadget_future =
        GadgetManager::new(substrate_gadget).map_err(|err| Error::GadgetManagerError { err });

    // Run both the network and the gadget together
    tokio::try_join!(network_future, gadget_future).map(|_| ())
}
