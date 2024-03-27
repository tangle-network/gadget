use lazy_static::lazy_static;
use prometheus::IntCounter;
use std::net::SocketAddr;

lazy_static! {
    pub static ref BYTES_RECEIVED: IntCounter =
        IntCounter::new("bytes_received", "Bytes Received").expect("metric can be created");
    pub static ref BYTES_SENT: IntCounter =
        IntCounter::new("bytes_sent", "Bytes Sent").expect("metric can be created");
}

#[derive(Debug, Clone)]
pub enum PrometheusConfig {
    Enabled { bind_addr: SocketAddr },
    Disabled,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self::Disabled
    }
}

pub async fn setup(_config: PrometheusConfig) -> Result<(), crate::Error> {
    Err(crate::Error::PrometheusError {
        err: "Prometheus is not yet supported on WASM targets".to_string(),
    })
}
