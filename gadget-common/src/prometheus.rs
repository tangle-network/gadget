use lazy_static::lazy_static;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use substrate_prometheus_endpoint::prometheus::{IntCounter, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref BYTES_RECEIVED: IntCounter =
        IntCounter::new("bytes_received", "Bytes Received").expect("metric can be created");
    pub static ref BYTES_SENT: IntCounter =
        IntCounter::new("bytes_sent", "Bytes Sent").expect("metric can be created");
}

#[derive(Debug, Clone)]
pub struct PrometheusConfig {
    pub prometheus_addr: SocketAddr,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            prometheus_addr: SocketAddr::from_str("0.0.0.0:9615").unwrap(),
        }
    }
}

pub async fn setup(config: PrometheusConfig) -> Result<(), crate::Error> {
    static HAS_REGISTERED: AtomicBool = AtomicBool::new(false);
    if HAS_REGISTERED.load(std::sync::atomic::Ordering::Relaxed) {
        return Ok(());
    }

    substrate_prometheus_endpoint::register(BYTES_RECEIVED.clone(), &REGISTRY).map_err(|err| {
        crate::Error::PrometheusError {
            err: err.to_string(),
        }
    })?;

    substrate_prometheus_endpoint::register(BYTES_SENT.clone(), &REGISTRY).map_err(|err| {
        crate::Error::PrometheusError {
            err: err.to_string(),
        }
    })?;

    substrate_prometheus_endpoint::init_prometheus(config.prometheus_addr, REGISTRY.clone())
        .await
        .map_err(|err| crate::Error::PrometheusError {
            err: err.to_string(),
        })?;

    HAS_REGISTERED.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
