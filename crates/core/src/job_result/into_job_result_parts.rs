use alloc::string::ToString;
use core::{convert::Infallible, fmt};

use super::IntoJobResult;
use crate::JobResult;
use crate::error::BoxError;
use crate::metadata::{MetadataMap, MetadataValue};

/// Trait for adding headers and extensions to a response.
///
/// # Example
///
/// ```rust
/// use blueprint_sdk::job_result::JobResultParts;
/// use blueprint_sdk::{IntoJobResult, IntoJobResultParts, JobResult};
///
/// // Hypothetical helper type for setting a single metadata value
/// struct SetMetadata<'a>(&'a str, &'a str);
///
/// impl<'a> IntoJobResultParts for SetMetadata<'a> {
///     type Error = String;
///
///     fn into_job_result_parts(
///         self,
///         mut res: JobResultParts,
///     ) -> Result<JobResultParts, Self::Error> {
///         if self.0 != "blueprint-sdk" {
///             return Err(format!("Invalid header name {}", self.0));
///         }
///
///         if let Some(metadata) = res.metadata_mut() {
///             metadata.insert(self.0.to_string(), self.1.to_string());
///         }
///
///         Ok(res)
///     }
/// }
///
/// // It's also recommended to implement `IntoJobResult` so `SetMetadata` can be used on its own as
/// // the response
/// impl<'a> IntoJobResult for SetMetadata<'a> {
///     fn into_job_result(self) -> Option<JobResult> {
///         // This gives an empty response with the header
///         (self, ()).into_job_result()
///     }
/// }
///
/// // We can now return `SetMetadata` in responses
/// //
/// // Note that returning `impl IntoJobResult` might be easier if the result has many parts to
/// // it. The return type is written out here for clarity.
/// async fn job() -> (SetMetadata<'static>, SetMetadata<'static>, &'static str) {
///     (
///         SetMetadata("server", "blueprint-sdk"),
///         SetMetadata("x-foo", "custom"),
///         "body",
///     )
/// }
///
/// // Or on its own as the whole response
/// async fn other_job() -> SetMetadata<'static> {
///     SetMetadata("x-foo", "custom")
/// }
/// ```
pub trait IntoJobResultParts {
    /// The type returned in the event of an error.
    ///
    /// This can be used to fallibly convert types into metadata.
    type Error: Into<BoxError> + 'static;

    /// Set parts of the response
    fn into_job_result_parts(self, res: JobResultParts) -> Result<JobResultParts, Self::Error>;
}

impl<T> IntoJobResultParts for Option<T>
where
    T: IntoJobResultParts,
{
    type Error = T::Error;

    fn into_job_result_parts(self, res: JobResultParts) -> Result<JobResultParts, Self::Error> {
        if let Some(inner) = self {
            inner.into_job_result_parts(res)
        } else {
            Ok(res)
        }
    }
}

/// Parts of a job result.
///
/// Used with [`IntoJobResult`].
#[derive(Debug)]
pub struct JobResultParts {
    pub(crate) res: JobResult,
}

impl JobResultParts {
    /// Gets a reference to the job result metadata.
    ///
    /// # Returns
    ///
    /// This will return `None` if the result is [`JobResult::Err`].
    #[must_use]
    pub fn metadata(&self) -> Option<&MetadataMap<MetadataValue>> {
        self.res.metadata()
    }

    /// Gets a mutable reference to the job result metadata.
    ///
    /// # Returns
    ///
    /// This will return `None` if the result is [`JobResult::Err`].
    #[must_use]
    pub fn metadata_mut(&mut self) -> Option<&mut MetadataMap<MetadataValue>> {
        self.res.metadata_mut()
    }
}

impl IntoJobResultParts for MetadataMap<MetadataValue> {
    type Error = BoxError;

    fn into_job_result_parts(self, mut res: JobResultParts) -> Result<JobResultParts, Self::Error> {
        if let Some(metadata) = res.metadata_mut() {
            metadata.extend(self);
        }

        Ok(res)
    }
}

impl<K, V, const N: usize> IntoJobResultParts for [(K, V); N]
where
    K: TryInto<&'static str>,
    <K as TryInto<&'static str>>::Error: core::error::Error + Send + Sync + 'static,
    V: TryInto<MetadataValue>,
    <V as TryInto<MetadataValue>>::Error: core::error::Error + Send + Sync + 'static,
{
    type Error = TryIntoMetadataError<K::Error, V::Error>;

    fn into_job_result_parts(self, mut res: JobResultParts) -> Result<JobResultParts, Self::Error> {
        let Some(metadata) = res.metadata_mut() else {
            return Ok(res);
        };

        for (key, value) in self {
            let key = key.try_into().map_err(TryIntoMetadataError::key)?;
            let value = value.try_into().map_err(TryIntoMetadataError::value)?;
            metadata.insert(key, value);
        }

        Ok(res)
    }
}

/// Error returned if converting a value to a metadata fails.
#[derive(Debug)]
pub struct TryIntoMetadataError<K, V> {
    kind: TryIntoMetadataErrorKind<K, V>,
}

impl<K, V> TryIntoMetadataError<K, V> {
    pub(super) fn key(err: K) -> Self {
        Self {
            kind: TryIntoMetadataErrorKind::Key(err),
        }
    }

    pub(super) fn value(err: V) -> Self {
        Self {
            kind: TryIntoMetadataErrorKind::Value(err),
        }
    }
}

#[derive(Debug)]
enum TryIntoMetadataErrorKind<K, V> {
    Key(K),
    Value(V),
}

impl<K, V> IntoJobResult for TryIntoMetadataError<K, V>
where
    K: fmt::Display,
    V: fmt::Display,
{
    fn into_job_result(self) -> Option<JobResult> {
        match self.kind {
            TryIntoMetadataErrorKind::Key(inner) => inner.to_string().into_job_result(),
            TryIntoMetadataErrorKind::Value(inner) => inner.to_string().into_job_result(),
        }
    }
}

impl<K, V> fmt::Display for TryIntoMetadataError<K, V>
where
    K: fmt::Display,
    V: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TryIntoMetadataErrorKind::Key(key) => {
                write!(f, "failed to convert key `{key}` to a header name")
            }
            TryIntoMetadataErrorKind::Value(val) => {
                write!(f, "failed to convert value `{val}` to a header value")
            }
        }
    }
}

impl<K, V> core::error::Error for TryIntoMetadataError<K, V>
where
    K: core::error::Error + 'static,
    V: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match &self.kind {
            TryIntoMetadataErrorKind::Key(inner) => Some(inner),
            TryIntoMetadataErrorKind::Value(inner) => Some(inner),
        }
    }
}

macro_rules! impl_into_job_result_parts {
    ( $($ty:ident),* $(,)? ) => {
        #[allow(non_snake_case)]
        impl<$($ty,)*> IntoJobResultParts for ($($ty,)*)
        where
            $(
            $ty: IntoJobResultParts,
            <$ty as IntoJobResultParts>::Error: core::error::Error + Send + Sync + Into<BoxError>,
            )*
        {
            type Error = BoxError;

            fn into_job_result_parts(self, res: JobResultParts) -> Result<JobResultParts, Self::Error> {
                let ($($ty,)*) = self;

                $(
                    let res = $ty.into_job_result_parts(res)?;
                )*

                Ok(res)
            }
        }
    }
}

all_the_tuples_no_last_special_case!(impl_into_job_result_parts);

impl IntoJobResultParts for () {
    type Error = Infallible;

    fn into_job_result_parts(self, res: JobResultParts) -> Result<JobResultParts, Self::Error> {
        Ok(res)
    }
}
