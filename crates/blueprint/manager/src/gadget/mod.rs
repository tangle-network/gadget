use gadget_std::collections::HashMap;
use gadget_std::sync::Arc;
use gadget_std::sync::atomic::AtomicBool;

pub type ActiveGadgets =
    HashMap<u64, HashMap<u64, (Arc<AtomicBool>, Option<tokio::sync::oneshot::Sender<()>>)>>;
pub mod native;
