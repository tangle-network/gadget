//! Polling-based block and event producer for EVM chains
//!
//! This module provides producers that poll an EVM node at regular intervals to fetch new blocks
//! and event logs. The producer implements a state machine pattern to manage the polling lifecycle
//! and maintain a buffer of job calls derived from event logs.
//!
//! # State Machine Flow
//! ```text
//! [Idle] ---(interval elapsed)---> [FetchingBlocks] ---(logs received)---> [Idle]
//!    ^                                    |
//!    |                                    |
//!    +------------(error occurs)----------+
//! ```
//!
//! # Polling Process
//! 1. Starts from configured block number
//! 2. Fetches logs in configured step sizes
//! 3. Respects confirmation depth for finality
//! 4. Converts logs to job calls
//! 5. Maintains a buffer of pending jobs

use alloy_provider::Provider;
use alloy_rpc_types::{Filter, Log};
use alloy_transport::TransportError;
use blueprint_core::JobCall;
use core::{
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use futures::{FutureExt, Stream};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::time::Sleep;

/// Configuration parameters for the polling producer
#[derive(Debug, Clone, Copy)]
pub struct PollingConfig {
    /// Starting block number for log collection
    pub start_block: u64,
    /// Interval between polling attempts
    pub poll_interval: Duration,
    /// Number of blocks to wait for finality
    pub confirmations: u64,
    /// Number of blocks to fetch in each polling cycle
    pub step: u64,
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            start_block: 0,
            poll_interval: Duration::from_secs(1),
            confirmations: 12,
            step: 1,
        }
    }
}

/// A streaming producer that polls an EVM chain for new events and converts them to job calls.
///
/// # State Machine
/// - `Idle`: Waits for configured interval before next polling attempt
/// - `FetchingBlocks`: Actively retrieving logs from the EVM node
///
/// # Buffer Management
/// Maintains an internal buffer of converted job calls to ensure smooth delivery
/// even when the node returns large batches of logs.
#[derive(Debug)]
pub struct PollingProducer<P: Provider> {
    provider: Arc<P>,
    filter: Filter,
    config: PollingConfig,
    state: PollingState,
    buffer: VecDeque<JobCall>,
}

/// Producer state for managing the polling lifecycle
enum PollingState {
    /// Fetching the current best block number
    FetchingBlockNumber(Pin<Box<dyn Future<Output = Result<u64, TransportError>> + Send>>),
    /// Fetching logs for a specific block range
    FetchingLogs(Pin<Box<dyn Future<Output = Result<Vec<Log>, TransportError>> + Send>>),
    /// Waiting for next polling interval
    Idle(Pin<Box<Sleep>>),
}

impl Debug for PollingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FetchingBlockNumber(_) => f.debug_tuple("FetchingBlockNumber").finish(),
            Self::FetchingLogs(_) => f.debug_tuple("FetchingLogs").finish(),
            Self::Idle(_) => f.debug_tuple("Idle").finish(),
        }
    }
}

impl<P: Provider> PollingProducer<P> {
    /// Creates a new polling producer with the specified configuration
    ///
    /// # Arguments
    /// * `provider` - The EVM provider to use for fetching logs
    /// * `config` - Configuration parameters for the polling behavior
    pub fn new(provider: Arc<P>, config: PollingConfig) -> Self {
        // Calculate initial block range accounting for confirmations
        let initial_start_block = config.start_block.saturating_sub(config.confirmations);
        let filter = Filter::new()
            .from_block(initial_start_block)
            .to_block(initial_start_block + config.step);

        blueprint_core::info!(
            start_block = initial_start_block,
            step = config.step,
            confirmations = config.confirmations,
            "Initializing polling producer"
        );

        Self {
            provider,
            config,
            filter,
            state: PollingState::Idle(Box::pin(tokio::time::sleep(Duration::from_micros(1)))),
            buffer: VecDeque::with_capacity(config.step as usize),
        }
    }
}

impl<P: Provider + 'static> Stream for PollingProducer<P> {
    type Item = Result<JobCall, TransportError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        // Serve from buffer if available
        if let Some(job) = this.buffer.pop_front() {
            if !this.buffer.is_empty() {
                cx.waker().wake_by_ref();
            }
            return Poll::Ready(Some(Ok(job)));
        }

        let provider = &this.provider;
        let state = &mut this.state;
        let buffer = &mut this.buffer;
        let filter = &mut this.filter;
        let config = &this.config;

        match state {
            PollingState::Idle(sleep) => {
                match sleep.poll_unpin(cx) {
                    Poll::Ready(_) => {
                        // Transition to fetching block number
                        blueprint_core::debug!(
                            "Polling interval elapsed, fetching current block number"
                        );
                        let fut = get_block_number(provider.clone());
                        *state = PollingState::FetchingBlockNumber(Box::pin(fut));
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
            PollingState::FetchingBlockNumber(fut) => {
                match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(current_block)) => {
                        // Calculate the highest block we can safely query considering confirmations
                        let safe_block = current_block.saturating_sub(config.confirmations);
                        let last_queried = filter.get_to_block().unwrap_or(config.start_block);

                        // Calculate next block range
                        let next_from_block = last_queried.saturating_add(1);
                        let proposed_to_block = last_queried.saturating_add(config.step);
                        let next_to_block = proposed_to_block.min(safe_block);

                        blueprint_core::debug!(
                            current_block,
                            safe_block,
                            next_from_block,
                            next_to_block,
                            "Calculated block range"
                        );

                        // Check if we have new blocks to process
                        if next_from_block > safe_block {
                            blueprint_core::debug!(
                                "No new blocks to process yet, waiting for next interval"
                            );
                            *state = PollingState::Idle(Box::pin(tokio::time::sleep(
                                config.poll_interval,
                            )));
                            cx.waker().wake_by_ref();
                            return Poll::Pending;
                        }

                        // Update filter for next range
                        *filter = Filter::new()
                            .from_block(next_from_block)
                            .to_block(next_to_block);

                        blueprint_core::info!(
                            from_block = next_from_block,
                            to_block = next_to_block,
                            current_block,
                            "Fetching logs for block range"
                        );

                        // Transition to fetching logs
                        let fut = get_logs(provider.clone(), filter.clone());
                        *state = PollingState::FetchingLogs(Box::pin(fut));
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                    Poll::Ready(Err(e)) => {
                        blueprint_core::error!(
                            error = ?e,
                            "Failed to fetch current block number, retrying after interval"
                        );
                        *state =
                            PollingState::Idle(Box::pin(tokio::time::sleep(config.poll_interval)));
                        Poll::Ready(Some(Err(e)))
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
            PollingState::FetchingLogs(fut) => {
                match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(logs)) => {
                        blueprint_core::info!(
                            logs_count = logs.len(),
                            from_block = ?filter.get_from_block(),
                            to_block = ?filter.get_to_block(),
                            "Successfully fetched logs"
                        );

                        // Convert logs to job calls and buffer them
                        let job_calls = super::logs_to_job_calls(logs);
                        buffer.extend(job_calls);

                        // Transition back to idle state
                        *state =
                            PollingState::Idle(Box::pin(tokio::time::sleep(config.poll_interval)));

                        // Try to serve first job from new batch
                        match buffer.pop_front() {
                            Some(job_call) => {
                                if !buffer.is_empty() {
                                    cx.waker().wake_by_ref();
                                }
                                Poll::Ready(Some(Ok(job_call)))
                            }
                            None => {
                                cx.waker().wake_by_ref();
                                Poll::Pending
                            }
                        }
                    }
                    Poll::Ready(Err(e)) => {
                        blueprint_core::error!(
                            error = ?e,
                            from_block = ?filter.get_from_block(),
                            to_block = ?filter.get_to_block(),
                            "Failed to fetch logs, retrying after interval"
                        );
                        *state =
                            PollingState::Idle(Box::pin(tokio::time::sleep(config.poll_interval)));
                        Poll::Ready(Some(Err(e)))
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

/// Fetches the current block number from the provider
async fn get_block_number<P: Provider>(provider: P) -> Result<u64, TransportError> {
    provider.get_block_number().await
}

/// Fetches logs from the provider for the specified filter range
async fn get_logs<P: Provider>(provider: P, filter: Filter) -> Result<Vec<Log>, TransportError> {
    blueprint_core::debug!(
        from_block = ?filter.get_from_block(),
        to_block = ?filter.get_to_block(),
        "Fetching logs from provider"
    );
    provider.get_logs(&filter).await
}
