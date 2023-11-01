use crate::gadget::message::GadgetProtocolMessage;
use tokio::sync::mpsc::UnboundedReceiver;

pub trait Network: Send + Sync {
    fn take_message_receiver(&mut self) -> Option<UnboundedReceiver<GadgetProtocolMessage>>;
}
