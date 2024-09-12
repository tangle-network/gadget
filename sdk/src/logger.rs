#[cfg(not(feature = "std"))]
use alloc::string::String;
use core::fmt::Display;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, trace, warn};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Logger {
    pub id: String,
}

impl Logger {
    pub fn trace<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            trace!(target: "gadget", "{msg}");
            return;
        }
        trace!(target: "gadget", "[{}] {msg}", &self.id);
    }

    pub fn debug<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            debug!(target: "gadget", "{msg}");
            return;
        }
        debug!(target: "gadget", "[{}] {msg}", &self.id);
    }

    pub fn info<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            info!(target: "gadget", "{msg}");
            return;
        }
        info!(target: "gadget", "[{}] {msg}", &self.id);
    }

    pub fn warn<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            warn!(target: "gadget", "{msg}");
            return;
        }
        warn!(target: "gadget", "[{}] {msg}", &self.id);
    }

    pub fn error<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            error!(target: "gadget", "{msg}");
            return;
        }
        error!(target: "gadget", "[{}] {msg}", &self.id);
    }
}

impl From<&'_ str> for Logger {
    fn from(id: &str) -> Self {
        Self::from(id.to_string())
    }
}

impl From<String> for Logger {
    fn from(id: String) -> Self {
        Self { id }
    }
}

pub fn setup_log() {
    use tracing_subscriber::fmt::SubscriberBuilder;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    if let Err(err) = SubscriberBuilder::default()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init()
    {
        eprintln!("Failed to initialize logger: {err:?}")
    }

    std::panic::set_hook(Box::new(|info| {
        log::error!(target: "gadget", "Panic occurred: {info:?}");
        std::process::exit(1);
    }));
}
