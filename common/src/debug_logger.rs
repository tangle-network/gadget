use std::fmt::Display;

#[derive(Clone)]
pub struct DebugLogger {
    pub peer_id: String,
}

impl DebugLogger {
    pub fn trace<T: Display>(&self, msg: T) {
        log::trace!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn debug<T: Display>(&self, msg: T) {
        log::debug!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn info<T: Display>(&self, msg: T) {
        log::info!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn warn<T: Display>(&self, msg: T) {
        log::warn!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn error<T: Display>(&self, msg: T) {
        log::error!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }
}
