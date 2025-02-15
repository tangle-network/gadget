use blueprint_job_router::IntoJobResult;
use blueprint_job_router::JobResult;
use bytes::Bytes;
use gadget_blueprint_serde::to_field;
use tangle_subxt::parity_scale_codec::Encode;

use blueprint_job_router::__define_rejection as define_rejection;

/// A simple wrapper that converts the result of a job call into a tangle specific result.
pub struct TangleResult<T>(pub T);

blueprint_job_router::__impl_deref!(TangleResult);

define_rejection! {
  #[body = "Failed to convert the job result into a tangle result"]
  /// A Rejection type for [`TangleResult`] when it fails to be converted into a job result.
  pub struct IntoJobResultFailed(Error);
}

impl<T> IntoJobResult for TangleResult<T>
where
    T: serde::Serialize,
{
    fn into_job_result(self) -> JobResult {
        to_field(self.0)
            .map(|f| Bytes::from(f.encode()))
            .map_err(IntoJobResultFailed::from_err)
            .into_job_result()
    }
}
