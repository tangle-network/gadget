//! Taken from <https://github.com/Layr-Labs/evmsdk-rs/blob/main/crates/logging/src/tracing_logger.rs>

use metrics::{Key, Label, describe_counter, describe_histogram};

#[derive(Debug)]
pub struct RpcCallsMetrics;

impl RpcCallsMetrics {
    #[must_use]
    pub fn new() -> Self {
        describe_histogram!(
            "evm_rpc_request_duration_seconds",
            "Duration of json-rpc <method> in seconds from Ethereum Execution client <client>"
        );
        describe_counter!(
            "evm_rpc_request_total",
            "Total of json-rpc <method> requests from Ethereum Execution client <client>"
        );

        Self
    }

    pub fn set_rpc_request_duration_seconds(
        &self,
        method: &str,
        client_version: &str,
        duration: f64,
    ) {
        let key = Key::from_parts("evm_rpc_request_duration_seconds", vec![
            Label::new("method ", method.to_string()),
            Label::new("client_version", client_version.to_string()),
        ]);

        metrics::histogram!(key.to_string()).record(duration);
    }

    pub fn set_rpc_request_total(
        &self,
        method: &str,
        client_version: &str,
        rpc_request_total: u64,
    ) {
        let key = Key::from_parts("evm_rpc_request_total", vec![
            Label::new("method", method.to_string()),
            Label::new("client_version", client_version.to_string()),
        ]);

        metrics::counter!(key.to_string()).absolute(rpc_request_total);
    }
}

impl Default for RpcCallsMetrics {
    fn default() -> Self {
        Self::new()
    }
}
