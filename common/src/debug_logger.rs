#[cfg(not(feature = "std"))]
use alloc::string::String;
use core::fmt::Display;

#[derive(Clone, Default, Debug)]
pub struct DebugLogger {
    pub id: String,
}

impl DebugLogger {
    pub fn trace<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::trace!(target: "gadget", "{msg}");
            return;
        }
        log::trace!(target: "gadget", "[{}] {msg}", &self.id);
    }

    pub fn debug<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::debug!(target: "gadget", "{msg}");
            return;
        }
        log::debug!(target: "gadget", "[{}] {msg}", &self.id);
    }

    pub fn info<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::info!(target: "gadget", "{msg}");
            return;
        }
        log::info!(target: "gadget", "[{}] {msg}", &self.id);
    }

    pub fn warn<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::warn!(target: "gadget", "{msg}");
            return;
        }
        log::warn!(target: "gadget", "[{}] {msg}", &self.id);
    }

    pub fn error<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::error!(target: "gadget", "{msg}");
            return;
        }
        log::error!(target: "gadget", "[{}] {msg}", &self.id);
    }
}

impl From<&'_ str> for DebugLogger {
    fn from(id: &str) -> Self {
        Self::from(id.to_string())
    }
}

impl From<String> for DebugLogger {
    fn from(id: String) -> Self {
        Self { id }
    }
}
