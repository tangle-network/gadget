#![allow(dead_code)]

pub mod metrics;
pub mod sourced;

pub use metrics::*;

/// Standard metrics for the gadget gossip service.
///
/// This module provides a set of standard metrics that are used to monitor
/// the behavior of the gadget gossip service. It includes counters for both
/// registered and expired messages, which can be used to track message
/// throughput and lifecycle within the service.
#[derive(Debug)]
pub struct Metrics {
    registered_messages: Counter<U64>,
    expired_messages: Counter<U64>,
}

impl Metrics {
    fn register(registry: &Registry) -> Result<Self, PrometheusError> {
        Ok(Self {
            registered_messages: register(
                Counter::new(
                    "gadget_gossip_registered_messages_total",
                    "Number of registered messages by the gossip service.",
                )?,
                registry,
            )?,
            expired_messages: register(
                Counter::new(
                    "gadget_gossip_expired_messages_total",
                    "Number of expired messages by the gossip service.",
                )?,
                registry,
            )?,
        })
    }
}
