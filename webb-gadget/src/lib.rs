use crate::gadget::network::Network;
use crate::gadget::{WebbGadgetModule, WebbModule};
use gadget_core::gadget::manager::{GadgetError, GadgetManager};
use gadget_core::gadget::substrate::{Client, SubstrateGadget};
use gadget_core::job_manager::WorkManagerError;
pub use sc_client_api::BlockImportNotification;
pub use sc_client_api::{Backend, FinalityNotification};
use sp_runtime::traits::Block;
use std::fmt::{Debug, Display, Formatter};

pub mod gadget;

#[derive(Debug)]
pub enum Error {
    RegistryCreateError { err: String },
    RegistrySendError { err: String },
    RegistryRecvError { err: String },
    RegistryListenError { err: String },
    GadgetManagerError { err: GadgetError },
    InitError { err: String },
    WorkManagerError { err: WorkManagerError },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

pub async fn run<C: Client<B, BE>, B: Block, BE: Backend<B>, N: Network, M: WebbGadgetModule<B>>(
    network: N,
    module: M,
    client: C,
) -> Result<(), Error> {
    let now = None;
    let webb_module = WebbModule::new(network, module, now);
    // Plug the module into the substrate gadget to interface the WebbGadget with Substrate
    let substrate_gadget = SubstrateGadget::new(client, webb_module);

    // Run the GadgetManager to execute the substrate gadget
    GadgetManager::new(substrate_gadget)
        .await
        .map_err(|err| Error::GadgetManagerError { err })
}
