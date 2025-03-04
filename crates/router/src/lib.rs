#![no_std]

extern crate alloc;

#[doc(hidden)]
pub mod __private {
    #[cfg(feature = "tracing")]
    pub use blueprint_core::__private::tracing;
}

mod boxed;
pub mod future;
mod into_make_service;
mod job_router;
mod nop;
pub mod routing;
pub mod service_ext;
mod util;

#[cfg(test)]
pub(crate) mod test_helpers;
#[cfg(test)]
mod tests;

pub use into_make_service::IntoMakeService;
pub use routing::Router;
pub use service_ext::ServiceExt;
