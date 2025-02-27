#[cfg(feature = "tracing")]
pub(crate) use tracing::debug;
#[cfg(not(feature = "tracing"))]
macro_rules! debug {
    ($($tt:tt)*) => {};
}
#[cfg(not(feature = "tracing"))]
pub(crate) use debug;

#[cfg(feature = "tracing")]
pub(crate) use tracing::trace;
#[cfg(not(feature = "tracing"))]
macro_rules! trace {
    ($($tt:tt)*) => {};
}
#[cfg(not(feature = "tracing"))]
pub(crate) use trace;
