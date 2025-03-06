//! A Smart extractor for all metadata in the current [`JobCall`].
//!
//!
//! [`JobCall`]: crate::JobCall

use core::convert::Infallible;

use crate::FromJobCallParts;
use crate::job_call::Parts as JobCallParts;
use crate::metadata::{MetadataMap, MetadataValue};

/// Extractor for all metadata in the current [`JobCall`].
///
/// [`JobCall`]: crate::JobCall
pub struct Metadata(pub MetadataMap<MetadataValue>);

impl<Ctx> FromJobCallParts<Ctx> for Metadata
where
    Ctx: Send + Sync + 'static,
{
    type Rejection = Infallible;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Ok(Metadata(parts.metadata.clone()))
    }
}
