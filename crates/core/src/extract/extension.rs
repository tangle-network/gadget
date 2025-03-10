//! A Smart Extractor that extracts any type from the [`Extensions`] of the current [`JobCall`].
//!
//! [`Extensions`]: crate::extensions::Extensions
//! [`JobCall`]: crate::JobCall

use core::convert::Infallible;

use alloc::format;

use crate::FromJobCallParts;
use crate::extract::OptionalFromJobCallParts;
use crate::job_call::Parts as JobCallParts;

/// A Specialized extractor for the [`Extensions`] of the current [`JobCall`].
///
/// Any type that is stored in the [`Extensions`] of the current [`JobCall`] can be extracted using this extractor.
///
/// For Optional extraction, you can use this extractor with with `Option<Extension<T>>` where `T` is the type you want to extract.
///
/// [`JobCall`]: crate::JobCall
/// [`Extensions`]: crate::extensions::Extensions
#[derive(Clone)]
pub struct Extension<T>(pub T);

crate::__impl_deref!(Extension);

crate::__define_rejection! {
    #[body = "Extension not found"]
    /// Rejection type used when the extension is not found in the current [`JobCall`].
    pub struct ExtensionNotFound(Error);
}

impl<Ctx, T> FromJobCallParts<Ctx> for Extension<T>
where
    T: Send + Sync + Clone + 'static,
    Ctx: Send + Sync + 'static,
{
    type Rejection = ExtensionNotFound;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        let ext = parts.extensions.get::<T>().cloned();
        match ext {
            Some(value) => Ok(Extension(value)),
            None => Err(ExtensionNotFound::from_err(format!(
                "trying to extract extension of type {} but it was not found",
                core::any::type_name::<T>()
            ))),
        }
    }
}

impl<Ctx, T> OptionalFromJobCallParts<Ctx> for Extension<T>
where
    T: Send + Sync + Clone + 'static,
    Ctx: Send + Sync + 'static,
{
    type Rejection = Infallible;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Option<Self>, Self::Rejection> {
        let ext = parts.extensions.get::<T>().cloned();
        match ext {
            Some(value) => Ok(Some(Extension(value))),
            None => Ok(None),
        }
    }
}
