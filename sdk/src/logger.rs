#[cfg(not(feature = "std"))]
use alloc::string::String;
use core::fmt::Display;

#[derive(Clone, Debug, Default)]
pub struct Logger {
    pub target: &'static str,
    pub id: String,
}

impl Logger {
    pub fn trace<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::trace!(target: self.target, "{msg}");
            return;
        }
        log::trace!(target: self.target, "[{}] {msg}", &self.id);
    }

    pub fn debug<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::debug!(target: self.target, "{msg}");
            return;
        }
        log::debug!(target: self.target, "[{}] {msg}", &self.id);
    }

    pub fn info<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::info!(target: self.target, "{msg}");
            return;
        }
        log::info!(target: self.target, "[{}] {msg}", &self.id);
    }

    pub fn warn<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::warn!(target: self.target, "{msg}");
            return;
        }
        log::warn!(target: self.target, "[{}] {msg}", &self.id);
    }

    pub fn error<T: Display>(&self, msg: T) {
        if self.id.is_empty() {
            log::error!(target: self.target, "{msg}");
            return;
        }
        log::error!(target: self.target, "[{}] {msg}", &self.id);
    }
}
