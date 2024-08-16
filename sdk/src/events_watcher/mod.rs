//! ## Overview
//!
//! Event watcher traits handle the syncing and listening of events for a given network.
//! The event watcher calls into a storage for handling of important state. The run implementation
//! of an event watcher polls for blocks. Implementations of the event watcher trait define an
//! action to take when the specified event is found in a block at the `handle_event` api.

use core::time::Duration;

use derive_more::Display;

/// EVM Event Watcher Module
pub mod evm;

/// Substrate Event Watcher Module
pub mod substrate;

/// Tangle Event Watcher Implementation
pub mod tangle;

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

// Copyright 2022 Webb Technologies Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// Target for logger
pub const TARGET: &str = "event_watcher";

/// The Kind of the Probe.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// When the Lifecycle of the event watcher changes, like starting or shutting down.
    #[display("lifecycle")]
    Lifecycle,
    /// event watcher Sync state on a specific chain/node.
    #[display("sync")]
    Sync,
    /// event watcher Transaction Queue state on a specific chain/node.
    #[display("tx_queue")]
    TxQueue,
    /// When the event watcher will retry to do something.
    #[display("retry")]
    Retry,
}
