pub use tokio;

/// The runtime trait that all runtimes must implement.
pub trait Runtime {
    /// Runs the given future to completion on the runtime.
    fn block_on<F>(&self, future: F) -> F::Output
    where
        F: gadget_std::future::Future;
}

/// The [`tokio`](https://crates.io/crates/tokio) runtime.
///
/// This will execute the benchmark using the `tokio` runtime.
#[derive(Debug, Clone, Copy)]
pub struct TokioRuntime;

impl Runtime for TokioRuntime {
    fn block_on<F>(&self, future: F) -> F::Output
    where
        F: gadget_std::future::Future,
    {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(future)
    }
}

/// A benchmarking harness.
#[derive(Debug)]
#[allow(dead_code)]
pub struct Bencher<R> {
    /// The runtime to use for running benchmarks.
    runtime: R,
    /// The time at which the benchmark started.
    started_at: gadget_std::time::Instant,
    /// The max number of cores for this benchmark.
    cores: usize,
}

/// The results of a benchmark.
///
/// This implements [`Display`] to provide a human-readable summary of the benchmark.
#[derive(Debug, Clone)]
pub struct BenchmarkSummary {
    /// The name of the benchmark.
    pub name: String,
    /// The job identifier.
    pub job_id: u8,
    /// The duration of the benchmark.
    pub elapsed: gadget_std::time::Duration,
    /// The number of cores the benchmark was run with.
    pub cores: usize,
    /// The amount of memory used by the benchmark (in bytes).
    pub ram_usage: u64,
}

impl<R: Runtime> Bencher<R> {
    /// Create a new benchmark harness.
    ///
    /// # Examples
    ///
    /// ```
    /// use gadget_benchmarking::{Bencher, TokioRuntime};
    ///
    /// const THREADS: usize = 4;
    ///
    /// let bencher = Bencher::new(THREADS, TokioRuntime);
    /// ```
    pub fn new(threads: usize, runtime: R) -> Self {
        Self {
            runtime,
            started_at: gadget_std::time::Instant::now(),
            cores: threads,
        }
    }

    /// Runs the given future on the [`Runtime`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gadget_benchmarking::{Bencher, TokioRuntime};
    ///
    /// const THREADS: usize = 4;
    ///
    /// let bencher = Bencher::new(THREADS, TokioRuntime);
    /// bencher.block_on(async {
    ///     // Do some work...
    /// });
    /// ```
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: gadget_std::future::Future,
    {
        self.runtime.block_on(future)
    }

    /// Ends the benchmark and returns a summary.
    ///
    /// # Panics
    ///
    /// This will panic in the event it cannot determine the process ID.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gadget_benchmarking::{Bencher, TokioRuntime};
    /// const THREADS: usize = 4;
    ///
    /// let bencher = Bencher::new(THREADS, TokioRuntime);
    /// bencher.block_on(async {
    ///     // Do some work...
    /// });
    ///
    /// let summary = bencher.stop("my_benchmark", 0);
    /// println!("{}", summary);
    /// ```
    #[cfg(feature = "std")] // TODO: Benchmark execution time for WASM?
    pub fn stop<N: ToString>(&self, name: N, job_id: u8) -> BenchmarkSummary {
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
            cores: self.cores,
            ram_usage,
        }
    }
}

impl gadget_std::fmt::Display for BenchmarkSummary {
    #[allow(clippy::cast_precision_loss)]
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
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
            self.name, self.job_id, self.elapsed, self.cores,
        )
    }
}
