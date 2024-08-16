//! ## Overview
//!
//! Event watcher traits handle the syncing and listening of events for a given network.
//! The event watcher calls into a storage for handling of important state. The run implementation
//! of an event watcher polls for blocks. Implementations of the event watcher trait define an
//! action to take when the specified event is found in a block at the `handle_event` api.

/// Error type for the event watcher module.
pub mod error;
pub use error::Error;

/// EVM Event Watcher Module
pub mod evm;

/// Substrate Event Watcher Module
pub mod substrate;

/// Tangle Event Watcher Implementation
pub mod tangle;

use core::time::Duration;

/// Constant with Max Retry Count is a backoff policy which always returns
/// a constant duration, until it exceeds the maximum retry count.
#[derive(Debug, Clone, Copy)]
pub struct ConstantWithMaxRetryCount {
    interval: Duration,
    max_retry_count: usize,
    count: usize,
}

impl ConstantWithMaxRetryCount {
    /// Creates a new Constant backoff with `interval` and `max_retry_count`.
    /// `interval` is the duration to wait between retries, and `max_retry_count` is the maximum
    /// number of retries, after which we return `None` to indicate that we should stop retrying.
    #[must_use]
    pub fn new(interval: Duration, max_retry_count: usize) -> Self {
        Self {
            interval,
            max_retry_count,
            count: 0,
        }
    }
}

impl backoff::backoff::Backoff for ConstantWithMaxRetryCount {
    fn next_backoff(&mut self) -> Option<Duration> {
        (self.count < self.max_retry_count).then(|| {
            self.count += 1;
            self.interval
        })
    }

    fn reset(&mut self) {
        self.count = 0;
    }
}
