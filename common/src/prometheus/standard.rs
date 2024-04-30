
use lazy_static::lazy_static;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use substrate_prometheus_endpoint::prometheus::{
    Gauge, Histogram, HistogramOpts, IntCounter, Registry,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref BYTES_RECEIVED: IntCounter =
        IntCounter::new("bytes_received", "Bytes Received").expect("metric can be created");
    pub static ref BYTES_SENT: IntCounter =
        IntCounter::new("bytes_sent", "Bytes Sent").expect("metric can be created");
    pub static ref JOBS_RUNNING: Gauge =
        Gauge::new("jobs_running", "Jobs Running").expect("metric can be created");
    pub static ref JOBS_STARTED: IntCounter =
        IntCounter::new("jobs_started", "Jobs Started").expect("metric can be created");
    pub static ref JOBS_COMPLETED: IntCounter =
        IntCounter::new("jobs_completed", "Completed jobs (agnostic to success)")
            .expect("metric can be created");
    pub static ref JOBS_COMPLETED_SUCCESS: IntCounter =
        IntCounter::new("jobs_completed_success", "Successfully completed jobs")
            .expect("metric can be created");
    pub static ref JOBS_COMPLETED_FAILED: IntCounter =
        IntCounter::new("jobs_completed_failed", "Unsuccessfully completed jobs")
            .expect("metric can be created");
    pub static ref TOKIO_ACTIVE_TASKS: Gauge =
        Gauge::new("tokio_tasks", "Number of active tasks").expect("metric can be created");
    pub static ref JOB_RUN_TIME: Histogram =
        Histogram::with_opts(HistogramOpts::new("job_runtime", "Job Runtime (s)"))
            .expect("metric can be created");
}

#[derive(Debug, Clone)]
pub enum PrometheusConfig {
    Enabled { bind_addr: SocketAddr },
    Disabled,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self::Enabled {
            bind_addr: SocketAddr::from_str("0.0.0.0:9615").unwrap(),
        }
    }
}

pub async fn setup(config: PrometheusConfig) -> Result<(), crate::Error> {
    if let PrometheusConfig::Enabled { bind_addr } = config {
        static HAS_REGISTERED: AtomicBool = AtomicBool::new(false);
        if HAS_REGISTERED.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }

        HAS_REGISTERED.store(true, std::sync::atomic::Ordering::SeqCst);

        substrate_prometheus_endpoint::register(BYTES_RECEIVED.clone(), &REGISTRY).map_err(
            |err| crate::Error::PrometheusError {
                err: err.to_string(),
            },
        )?;

        substrate_prometheus_endpoint::register(BYTES_SENT.clone(), &REGISTRY).map_err(|err| {
            crate::Error::PrometheusError {
                err: err.to_string(),
            }
        })?;

        substrate_prometheus_endpoint::init_prometheus(bind_addr, REGISTRY.clone())
            .await
            .map_err(|err| crate::Error::PrometheusError {
                err: err.to_string(),
            })?;
    }

    Ok(())
}
