use blueprint_job_router::{FromJobCall, JobCall};

use blueprint_job_router::__define_rejection as define_rejection;
use gadget_blueprint_serde::{from_field, Field};
use tangle_subxt::parity_scale_codec::Decode;

define_rejection! {
  #[body = "Failed to extract the arguments from the job call"]
  /// A Rejection type for [`TangleArgs`] when it fails to extract the arguments from the job call.
  pub struct ArgsExtractionFailed(Error);
}

/// An extractor for the arguments of a job call that deserializes the arguments from the job call body.
pub struct TangleArgs<T>(pub T);

blueprint_job_router::__impl_deref!(TangleArgs);

impl<T, Ctx> FromJobCall<Ctx> for TangleArgs<T>
where
    Ctx: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = ArgsExtractionFailed;

    async fn from_job_call(call: JobCall, _ctx: &Ctx) -> Result<Self, Self::Rejection> {
        let fields =
            Field::decode(&mut call.body().as_ref()).map_err(ArgsExtractionFailed::from_err)?;
        let args = from_field(fields).map_err(ArgsExtractionFailed::from_err)?;
        Ok(TangleArgs(args))
    }
}
