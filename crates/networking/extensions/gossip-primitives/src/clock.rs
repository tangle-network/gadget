//! Monotonic clock abstraction for TTL-style bookkeeping.
//!
//! With the `std` feature this re-exports [`std::time::Instant`]. Under `no_std` there
//! is no OS-provided monotonic clock, so the type is a no-op placeholder: `elapsed()`
//! always returns zero, meaning TTL-based expiry never fires and entries leave caches
//! only through LRU eviction. This keeps the crate's data structures `no_std`-clean
//! without inventing a clock source the host platform must supply.
//!
//! Ordering and subtraction are still implemented for the `no_std` placeholder so
//! generic containers (`BTreeMap<Instant, _>`) and range queries keep compiling; with
//! every element equal, those operations simply order nothing before anything else.

#[cfg(feature = "std")]
pub use std::time::Instant;

#[cfg(not(feature = "std"))]
use core::time::Duration;

#[cfg(not(feature = "std"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Instant {
    /// Placeholder timestamp; carries no information without a clock.
    NoClock,
}

#[cfg(not(feature = "std"))]
impl Instant {
    /// No-op without a clock: returns [`Self::NoClock`].
    #[must_use]
    pub fn now() -> Self {
        Self::NoClock
    }

    /// Without a clock, no elapsed time can be measured.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        Duration::ZERO
    }

    /// Without a clock there is no timeline: the cutoff equals "now".
    #[must_use]
    pub fn checked_sub(&self, _rhs: Duration) -> Option<Self> {
        Some(*self)
    }
}

#[cfg(not(feature = "std"))]
impl core::ops::Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Instant {
        self.checked_sub(rhs)
            .expect("duration subtraction cannot fail without a clock")
    }
}
