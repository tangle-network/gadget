use blueprint_job_router::{
    __composite_rejection as composite_rejection, __define_rejection as define_rejection,
};
use blueprint_job_router::{job_call::Parts as JobCallParts, FromJobCallParts};

/// Extracts the current service id from the job call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceId(pub u64);

impl ServiceId {
    pub const METADATA_KEY: &'static str = "X-TANGLE-SERVICE-ID";
}

blueprint_job_router::__impl_deref!(ServiceId: u64);
blueprint_job_router::__impl_from!(u64, ServiceId);

define_rejection! {
  #[body = "No ServiceId found in the metadata"]
  /// A Rejection type for [`ServiceId`] when it is missing from the Metadata.
  pub struct MissingServiceId;
}

define_rejection! {
  #[body = "The service id in the metadata is not a valid integer"]
  /// A Rejection type for [`ServiceId`] when it is not a valid u64.
  pub struct InvalidServiceId;
}

composite_rejection! {
    /// Rejection used for [`ServiceId`].
    ///
    /// Contains one variant for each way the [`ServiceId`] extractor
    /// can fail.
    pub enum ServiceIdRejection {
        MissingServiceId,
        InvalidServiceId,
    }
}

impl TryFrom<&mut JobCallParts> for ServiceId {
    type Error = ServiceIdRejection;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let service_id_raw = parts
            .metadata
            .get(Self::METADATA_KEY)
            .ok_or(MissingServiceId)?;
        let service_id = service_id_raw.try_into().map_err(|_| InvalidServiceId)?;
        Ok(ServiceId(service_id))
    }
}

impl<Ctx> FromJobCallParts<Ctx> for ServiceId
where
    Ctx: Send + Sync,
{
    type Rejection = ServiceIdRejection;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}
