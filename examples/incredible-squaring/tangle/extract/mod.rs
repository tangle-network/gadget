mod call_id;
use blueprint_sdk::{FromJobCall, JobCall};
pub use call_id::*;

pub struct Args<T>(pub T);

impl<T, Ctx> FromJobCall<Ctx> for Args<T>
where
    Ctx: Send + Sync,
{
    type Rejection = ();

    async fn from_job_call(call: JobCall, ctx: &Ctx) -> Result<Self, Self::Rejection> {
        todo!()
    }
}
