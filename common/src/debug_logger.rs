use std::fmt::Display;

#[derive(Clone, Default)]
pub struct DebugLogger {
    pub peer_id: String,
}

impl DebugLogger {
    pub fn trace<T: Display>(&self, msg: T) {
        if self.peer_id.is_empty() {
            log::trace!(target: "gadget", "{msg}");
            return;
        }
        log::trace!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn debug<T: Display>(&self, msg: T) {
        if self.peer_id.is_empty() {
            log::debug!(target: "gadget", "{msg}");
            return;
        }
        log::debug!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn info<T: Display>(&self, msg: T) {
        if self.peer_id.is_empty() {
            log::info!(target: "gadget", "{msg}");
            return;
        }
        log::info!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn warn<T: Display>(&self, msg: T) {
        if self.peer_id.is_empty() {
            log::warn!(target: "gadget", "{msg}");
            return;
        }
        log::warn!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }

    pub fn error<T: Display>(&self, msg: T) {
        if self.peer_id.is_empty() {
            log::error!(target: "gadget", "{msg}");
            return;
        }
        log::error!(target: "gadget", "[{}] {msg}", &self.peer_id);
    }
}
