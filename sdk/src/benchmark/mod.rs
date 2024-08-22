pub use tokio;
/// The runtime trait that all runtimes must implement.
pub trait Runtime {
    /// Runs the given future to completion on the runtime.
    fn block_on<F>(&self, future: F) -> F::Output
    where
        F: std::future::Future;
}

#[derive(Debug, Clone, Copy)]
pub struct TokioRuntime;

impl Runtime for TokioRuntime {
    fn block_on<F>(&self, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(future)
    }
}

/// A benchmarking harness.
#[derive(Debug)]
pub struct Bencher<R> {
    /// The runtime to use for running benchmarks.
    runtime: R,
    /// The time at which the benchmark started.
    started_at: std::time::Instant,
    /// The number of threads to run the benchmark with.
    threads: usize,
}

#[derive(Debug, Clone)]
pub struct BenchmarkSummary {
    /// The name of the benchmark.
    pub name: String,
    /// The job identifier.
    pub job_id: u8,
    /// The duration of the benchmark.
    pub elapsed: std::time::Duration,
    /// The number of threads the benchmark was run with.
    pub threads: usize,
    /// The amount of memory used by the benchmark (in bytes).
    pub ram_usage: u64,
}

impl<R: Runtime> Bencher<R> {
    pub fn new(threads: usize, runtime: R) -> Self {
        Self {
            runtime,
            started_at: std::time::Instant::now(),
            threads,
        }
    }

    /// Runs the given future on the runtime.
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.runtime.block_on(future)
    }

    /// Stops the benchmark and returns a summary of the benchmark.
    pub fn stop(&self, name: &str, job_id: u8) -> BenchmarkSummary {
        let pid = sysinfo::get_current_pid().expect("Failed to get current process ID");
        let s = sysinfo::System::new_all();
        let process = s
            .process(pid)
            .expect("Failed to get current process from the system");
        let ram_usage = process.memory();
        BenchmarkSummary {
            name: name.to_string(),
            job_id,
            elapsed: self.started_at.elapsed(),
            threads: self.threads,
            ram_usage,
        }
    }
}

impl std::fmt::Display for BenchmarkSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const KB: f32 = 1024.00;
        const MB: f32 = 1024.00 * KB;
        const GB: f32 = 1024.00 * MB;
        let ram_usage = self.ram_usage as f32;
        let (ram_usage, unit) = if ram_usage < KB {
            (ram_usage, "B")
        } else if ram_usage < MB {
            (ram_usage / KB, "KB")
        } else if ram_usage < GB {
            (ram_usage / MB, "MB")
        } else {
            (ram_usage / GB, "GB")
        };

        write!(
            f,
            "Benchmark: {}\nJob ID: {}\nElapsed: {:?}\nvCPU: {}\nRAM Usage: {ram_usage:.2} {unit}\n",
            self.name, self.job_id, self.elapsed, self.threads,
        )
    }
}
