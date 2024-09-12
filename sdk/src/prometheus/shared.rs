use prometheus::{Gauge, Histogram, HistogramOpts, IntCounter, Registry};
use std::sync::LazyLock;

/// The global Prometheus metrics registry.
pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

pub static BYTES_RECEIVED: LazyLock<IntCounter> = LazyLock::new(|| {
    IntCounter::new("bytes_received", "Bytes Received").expect("metric can be created")
});
pub static BYTES_SENT: LazyLock<IntCounter> =
    LazyLock::new(|| IntCounter::new("bytes_sent", "Bytes Sent").expect("metric can be created"));
pub static JOBS_RUNNING: LazyLock<Gauge> =
    LazyLock::new(|| Gauge::new("jobs_running", "Jobs Running").expect("metric can be created"));
pub static JOBS_STARTED: LazyLock<IntCounter> = LazyLock::new(|| {
    IntCounter::new("jobs_started", "Jobs Started").expect("metric can be created")
});
pub static JOBS_COMPLETED: LazyLock<IntCounter> = LazyLock::new(|| {
    IntCounter::new("jobs_completed", "Completed jobs (agnostic to success)")
        .expect("metric can be created")
});
pub static JOBS_COMPLETED_SUCCESS: LazyLock<IntCounter> = LazyLock::new(|| {
    IntCounter::new("jobs_completed_success", "Successfully completed jobs")
        .expect("metric can be created")
});
pub static JOBS_COMPLETED_FAILED: LazyLock<IntCounter> = LazyLock::new(|| {
    IntCounter::new("jobs_completed_failed", "Unsuccessfully completed jobs")
        .expect("metric can be created")
});
pub static TOKIO_ACTIVE_TASKS: LazyLock<Gauge> = LazyLock::new(|| {
    Gauge::new("tokio_tasks", "Number of active tasks").expect("metric can be created")
});
pub static JOB_RUN_TIME: LazyLock<Histogram> = LazyLock::new(|| {
    Histogram::with_opts(HistogramOpts::new("job_runtime", "Job Runtime (s)"))
        .expect("metric can be created")
});
