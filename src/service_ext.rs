use crate::routing::IntoMakeService;
use tower::Service;

/// Extension trait that adds additional methods to any [`Service`].
pub trait ServiceExt<R>: Service<R> + Sized {
    /// Convert this service into a [`MakeService`], that is a [`Service`] whose
    /// response is another service.
    ///
    /// This is commonly used when applying middleware around an entire [`Router`]. See ["Rewriting
    /// request URI in middleware"] for more details.
    ///
    /// [`MakeService`]: tower::make::MakeService
    /// ["Rewriting request URI in middleware"]: crate::middleware#rewriting-request-uri-in-middleware
    /// [`Router`]: crate::Router
    fn into_make_service(self) -> IntoMakeService<Self>;
}

impl<S, R> ServiceExt<R> for S
where
    S: Service<R> + Sized,
{
    fn into_make_service(self) -> IntoMakeService<Self> {
        IntoMakeService::new(self)
    }
}
