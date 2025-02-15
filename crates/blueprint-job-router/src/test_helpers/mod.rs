#![allow(clippy::disallowed_names)]

#[cfg(test)]
pub(crate) mod tracing_helpers;

#[cfg(test)]
pub(crate) fn assert_send<T: Send>() {}
#[cfg(test)]
pub(crate) fn assert_sync<T: Sync>() {}

#[allow(dead_code)]
pub(crate) struct NotSendSync(*const ());
