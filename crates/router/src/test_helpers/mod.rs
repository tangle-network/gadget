#![allow(clippy::disallowed_names)]

use blueprint_core::__private::tracing::level_filters::LevelFilter;

pub(crate) fn assert_send<T: Send>() {}
pub(crate) fn assert_sync<T: Sync>() {}
#[allow(dead_code)]
pub(crate) struct NotSendSync(*const ());

pub fn setup_log() {
    use tracing_subscriber::util::SubscriberInitExt;

    let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
        .without_time()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .finish()
        .try_init();
}
