/// A [`trace`] log with the target `"gadget"`
///
/// [`trace`]: tracing::trace
#[macro_export]
macro_rules! trace {
    ($($tt:tt)*) => {
        $crate::tracing::trace!(target: "gadget", $($tt)*)
    }
}

/// A [`debug`] log with the target `"gadget"`
///
/// [`debug`]: tracing::debug
#[macro_export]
macro_rules! debug {
    ($($tt:tt)*) => {
        $crate::tracing::debug!(target: "gadget", $($tt)*)
    }
}

/// An [`error`] log with the target `"gadget"`
///
/// [`error`]: tracing::error
#[macro_export]
macro_rules! error {
    ($($tt:tt)*) => {
        $crate::tracing::error!(target: "gadget", $($tt)*)
    }
}

/// A [`warn`] log with the target `"gadget"`
///
/// [`warn`]: tracing::warn
#[macro_export]
macro_rules! warn {
    ($($tt:tt)*) => {
        $crate::tracing::warn!(target: "gadget", $($tt)*)
    }
}

/// An [`info`] log with the target `"gadget"`
///
/// [`info`]: tracing::info
#[macro_export]
macro_rules! info {
    ($($tt:tt)*) => {
        $crate::tracing::info!(target: "gadget", $($tt)*)
    }
}

/// Sets up the logging for any crate
pub fn setup_log() {
    use tracing_subscriber::util::SubscriberInitExt;

    let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
        .without_time()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish()
        .try_init();
}
