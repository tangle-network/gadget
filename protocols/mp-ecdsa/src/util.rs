use std::fmt::Display;

#[derive(Clone)]
pub struct DebugLogger;

impl DebugLogger {
    pub fn trace<T: Display>(&self, msg: T) {
        log::trace!(target: "gadget", "{msg}");
    }

    pub fn debug<T: Display>(&self, msg: T) {
        log::debug!(target: "gadget", "{msg}");
    }

    pub fn info<T: Display>(&self, msg: T) {
        log::info!(target: "gadget", "{msg}");
    }

    pub fn warn<T: Display>(&self, msg: T) {
        log::warn!(target: "gadget", "{msg}");
    }

    pub fn error<T: Display>(&self, msg: T) {
        log::error!(target: "gadget", "{msg}");
    }
}
