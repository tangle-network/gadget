use lazy_static::lazy_static;
use prometheus::{
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