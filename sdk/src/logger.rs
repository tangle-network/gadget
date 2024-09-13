#[cfg(not(feature = "std"))]
use alloc::string::String;
use core::fmt::Display;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, trace, warn};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Logger {
    pub id: String,
}

impl From<&str> for Logger {
    fn from(id: &str) -> Self {
        Logger::from(id.to_string())
    }
}

impl From<String> for Logger {
    fn from(id: String) -> Self {
        Logger { id }
    }
}

impl Logger {
    /// Calls the [`log::trace!`] macro, using the logger's configurations.
    pub fn trace<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            trace!(target: "gadget", "{msg}");
            return;
        }
        trace!(target: "gadget", "[{}] {msg}", &self.id);
    }

    /// Calls the [`log::debug!`] macro, using the logger's configurations.
    pub fn debug<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            debug!(target: "gadget", "{msg}");
            return;
        }
        debug!(target: "gadget", "[{}] {msg}", &self.id);
    }

    /// Calls the [`log::info!`] macro, using the logger's configurations.
    pub fn info<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            info!(target: "gadget", "{msg}");
            return;
        }
        info!(target: "gadget", "[{}] {msg}", &self.id);
    }

    /// Calls the [`log::warn!`] macro, using the logger's configurations.
    pub fn warn<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            warn!(target: "gadget", "{msg}");
            return;
        }
        warn!(target: "gadget", "[{}] {msg}", &self.id);
    }

    /// Calls the [`log::error!`] macro, using the logger's configurations.
    pub fn error<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            error!(target: "gadget", "{msg}");
            return;
        }
        error!(target: "gadget", "[{}] {msg}", &self.id);
    }
}
