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

use alloc::collections::VecDeque;
use alloc::sync::Arc;
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
use futures::Stream;
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

        blueprint_core::trace!(
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

        loop {
            // Serve from buffer if available
            if let Some(job) = this.buffer.pop_front() {
                if !this.buffer.is_empty() {
                    cx.waker().wake_by_ref();
                }
                return Poll::Ready(Some(Ok(job)));
            }

            match this.state {
                PollingState::Idle(ref mut fut) => match fut.as_mut().poll(cx) {
                    Poll::Ready(_) => {
                        // Transition to fetching block number
                        blueprint_core::trace!(
                            "Polling interval elapsed, fetching current block number"
                        );
                        let fut = get_block_number(this.provider.clone());
                        this.state = PollingState::FetchingBlockNumber(Box::pin(fut));
                        continue;
                    }
                    Poll::Pending => return Poll::Pending,
                },
                PollingState::FetchingBlockNumber(ref mut fut) => match fut.as_mut().poll(cx) {
                    Poll::Ready(Ok(current_block)) => {
                        // Calculate the highest block we can safely query considering confirmations
                        let safe_block = current_block.saturating_sub(this.config.confirmations);
                        let last_queried = this
                            .filter
                            .get_to_block()
                            .unwrap_or(this.config.start_block);

                        // Calculate next block range
                        let next_from_block = last_queried.saturating_add(1);
                        let proposed_to_block = last_queried.saturating_add(this.config.step);
                        let next_to_block = proposed_to_block.min(safe_block);

                        blueprint_core::trace!(
                            current_block,
                            safe_block,
                            next_from_block,
                            next_to_block,
                            "Calculated block range"
                        );

                        // Check if we have new blocks to process
                        if next_from_block > safe_block {
                            blueprint_core::trace!(
                                "No new blocks to process yet, waiting for next interval"
                            );
                            this.state = PollingState::Idle(Box::pin(tokio::time::sleep(
                                this.config.poll_interval,
                            )));
                            continue;
                        }

                        // Update filter for next range
                        this.filter = Filter::new()
                            .from_block(next_from_block)
                            .to_block(next_to_block);

                        blueprint_core::trace!(
                            from_block = next_from_block,
                            to_block = next_to_block,
                            current_block,
                            "Fetching logs for block range"
                        );

                        // Transition to fetching logs
                        let fut = get_logs(this.provider.clone(), this.filter.clone());
                        this.state = PollingState::FetchingLogs(Box::pin(fut));
                        continue;
                    }
                    Poll::Ready(Err(e)) => {
                        blueprint_core::error!(
                            error = ?e,
                            "Failed to fetch current block number, retrying after interval"
                        );
                        this.state = PollingState::Idle(Box::pin(tokio::time::sleep(
                            this.config.poll_interval,
                        )));
                        return Poll::Ready(Some(Err(e)));
                    }
                    Poll::Pending => return Poll::Pending,
                },
                PollingState::FetchingLogs(ref mut fut) => match fut.as_mut().poll(cx) {
                    Poll::Ready(Ok(logs)) => {
                        blueprint_core::trace!(
                            logs_count = logs.len(),
                            from_block = ?this.filter.get_from_block(),
                            to_block = ?this.filter.get_to_block(),
                            "Successfully fetched logs"
                        );

                        // Convert logs to job calls and buffer them
                        let job_calls = super::logs_to_job_calls(logs);
                        this.buffer.extend(job_calls);

                        // Transition back to idle state
                        this.state = PollingState::Idle(Box::pin(tokio::time::sleep(
                            this.config.poll_interval,
                        )));

                        continue;
                    }
                    Poll::Ready(Err(e)) => {
                        blueprint_core::error!(
                            error = ?e,
                            from_block = ?this.filter.get_from_block(),
                            to_block = ?this.filter.get_to_block(),
                            "Failed to fetch logs, retrying after interval"
                        );
                        this.state = PollingState::Idle(Box::pin(tokio::time::sleep(
                            this.config.poll_interval,
                        )));
                        return Poll::Ready(Some(Err(e)));
                    }
                    Poll::Pending => return Poll::Pending,
                },
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
    blueprint_core::trace!(
        from_block = ?filter.get_from_block(),
        to_block = ?filter.get_to_block(),
        "Fetching logs from provider"
    );
    let logs = provider.get_logs(&filter).await?;
    blueprint_core::trace!(
        from_block = ?filter.get_from_block(),
        to_block = ?filter.get_to_block(),
        logs_count = logs.len(),
        "Fetched logs from provider"
    );
    Ok(logs)
}
