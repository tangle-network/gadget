#![cfg_attr(not(feature = "std"), no_std)]

pub use tracing;
use tracing::level_filters::LevelFilter;
pub use tracing_subscriber;

/// A [`trace`] log with the target `"gadget"`
///
/// [`trace`]: tracing::trace
#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($tt:tt)*) => {
        $crate::tracing::trace!(target: $target, $($tt)*)
    };
    ($($tt:tt)*) => {
        $crate::tracing::trace!(target: "gadget", $($tt)*)
    }
}

/// A [`debug`] log with the target `"gadget"`
///
/// [`debug`]: tracing::debug
#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($tt:tt)*) => {
        $crate::tracing::debug!(target: $target, $($tt)*)
    };
    ($($tt:tt)*) => {
        $crate::tracing::debug!(target: "gadget", $($tt)*)
    }
}

/// An [`info`] log with the target `"gadget"`
///
/// [`info`]: tracing::info
#[macro_export]
macro_rules! info {
    (target: $target:expr, $($tt:tt)*) => {
        $crate::tracing::info!(target: $target, $($tt)*)
    };
    ($($tt:tt)*) => {
        $crate::tracing::info!(target: "gadget", $($tt)*)
    }
}

/// A [`warn`] log with the target `"gadget"`
///
/// [`warn`]: tracing::warn
#[macro_export]
macro_rules! warn {
    (target: $target:expr, $($tt:tt)*) => {
        $crate::tracing::warn!(target: $target, $($tt)*)
    };
    ($($tt:tt)*) => {
        $crate::tracing::warn!(target: "gadget", $($tt)*)
    }
}

/// An [`error`] log with the target `"gadget"`
///
/// [`error`]: tracing::error
#[macro_export]
macro_rules! error {
    (target: $target:expr, $($tt:tt)*) => {
        $crate::tracing::error!(target: $target, $($tt)*)
    };
    ($($tt:tt)*) => {
        $crate::tracing::error!(target: "gadget", $($tt)*)
    }
}

/// Sets up the logging for any crate
pub fn setup_log() {
    use tracing_subscriber::util::SubscriberInitExt;

    let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
        .without_time()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
        .with_env_filter(tracing_subscriber::EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy())
        .finish()
        .try_init();
}
