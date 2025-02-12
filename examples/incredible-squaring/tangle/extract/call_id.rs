use blueprint_sdk::{
    __composite_rejection as composite_rejection, __define_rejection as define_rejection,
};
use blueprint_sdk::{job_call::Parts as JobCallParts, FromJobCallParts};

/// Extracts the current call id from the job call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallId(pub u64);

impl CallId {
    pub const METADATA_KEY: &'static str = "X-TANGLE-CALL-ID";
}

blueprint_sdk::__impl_deref!(CallId: u64);
blueprint_sdk::__impl_from!(u64, CallId);

define_rejection! {
  #[body = "No CallId found in the metadata"]
  /// A Rejection type for [`CallId`] when it is missing from the Metadata.
  pub struct MissingCallId;
}

blueprint_sdk::__define_rejection! {
  #[body = "The call id in the metadata is not a valid integer"]
  /// A Rejection type for [`CallId`] when it is not a valid u64.
  pub struct InvalidCallId;
}

composite_rejection! {
    /// Rejection used for [`CallId`].
    ///
    /// Contains one variant for each way the [`Form`](super::Form) extractor
    /// can fail.
    pub enum CallIdRejection {
        MissingCallId,
        InvalidCallId,
    }
}

impl<Ctx> FromJobCallParts<Ctx> for CallId
where
    Ctx: Send + Sync,
{
    type Rejection = CallIdRejection;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        let call_id_raw = parts
            .metadata
            .get(Self::METADATA_KEY)
            .ok_or(MissingCallId)?;
        let call_id = call_id_raw.try_into().map_err(|_| InvalidCallId)?;
        Ok(CallId(call_id))
    }
}
