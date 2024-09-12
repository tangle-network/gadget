#[cfg(not(feature = "std"))]
use alloc::string::String;
use core::fmt::Display;
use serde::{Deserialize, Serialize};

const DEFAULT_TARGET: &str = "gadget";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Logger {
    pub target: String,
    pub id: String,
}

impl From<&str> for Logger {
    fn from(id: &str) -> Self {
        Logger {
            target: DEFAULT_TARGET.to_string(),
            id: id.to_string(),
        }
    }
}

impl From<String> for Logger {
    fn from(id: String) -> Self {
        Logger {
            target: DEFAULT_TARGET.to_string(),
            id,
        }
    }
}

impl Logger {
    /// Method to change the target of the Logger. Defaults to `gadget`.
    pub fn set_target(&mut self, new_target: &str) {
        self.target = new_target.to_string(); // Convert &str to String
    }

    /// Calls the [`log::trace!`] macro, using the logger's configurations.
    pub fn trace<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::trace!(target: &self.target, "{msg}");
            return;
        }
        log::trace!(target: &self.target, "[{}] {msg}", &self.id);
    }

    /// Calls the [`log::debug!`] macro, using the logger's configurations.
    pub fn debug<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::debug!(target: &self.target, "{msg}");
            return;
        }
        log::debug!(target: &self.target, "[{}] {msg}", &self.id);
    }

    /// Calls the [`log::info!`] macro, using the logger's configurations.
    pub fn info<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::info!(target: &self.target, "{msg}");
            return;
        }
        log::info!(target: &self.target, "[{}] {msg}", &self.id);
    }

    /// Calls the [`log::warn!`] macro, using the logger's configurations.
    pub fn warn<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::warn!(target: &self.target, "{msg}");
            return;
        }
        log::warn!(target: &self.target, "[{}] {msg}", &self.id);
    }

    /// Calls the [`log::error!`] macro, using the logger's configurations.
    pub fn error<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::error!(target: &self.target, "{msg}");
            return;
        }
        log::error!(target: &self.target, "[{}] {msg}", &self.id);
    }
}

pub fn setup_log() {
    use tracing_subscriber::fmt::SubscriberBuilder;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    let _ = SubscriberBuilder::default()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init();

    std::panic::set_hook(Box::new(|info| {
        log::error!(target: "gadget", "Panic occurred: {info:?}");
    }));
}
