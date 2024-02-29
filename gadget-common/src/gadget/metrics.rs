#![allow(dead_code)]

use substrate_prometheus_endpoint::{register, Counter, PrometheusError, Registry, U64};

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
