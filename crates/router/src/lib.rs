#![no_std]

extern crate alloc;

#[doc(hidden)]
pub mod __private {
    #[cfg(feature = "tracing")]
    pub use tracing;
}

mod boxed;
pub mod future;
mod into_make_service;
mod nop;
mod path_router;
pub mod routing;
pub mod service_ext;
#[cfg(test)]
pub(crate) mod test_helpers;
mod util;

pub use into_make_service::IntoMakeService;
pub use routing::Router;
pub use service_ext::ServiceExt;
