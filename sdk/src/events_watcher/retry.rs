use core::time::Duration;

/// A backoff policy which always returns a constant duration, with no maximum retry count.
#[derive(Debug, Clone, Copy)]
pub struct UnboundedConstantBuilder {
    interval: Duration,
}

impl UnboundedConstantBuilder {
    /// Creates a new unbounded Constant backoff
    ///
    /// * `interval` is the duration to wait between retries.
    #[must_use]
    #[allow(dead_code)]
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }
}

impl backon::BackoffBuilder for UnboundedConstantBuilder {
    type Backoff = UnboundedConstantBuilder;

    fn build(self) -> Self::Backoff {
        self
    }
}

impl Iterator for UnboundedConstantBuilder {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.interval)
    }
}
