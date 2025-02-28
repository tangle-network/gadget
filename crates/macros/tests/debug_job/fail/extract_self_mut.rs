use blueprint_macros::debug_job;
use blueprint_sdk::{FromJobCall, JobCall};

struct A;

impl<Ctx> FromJobCall<Ctx> for A
where
    Ctx: Send + Sync,
{
    type Rejection = ();

    async fn from_job_call(_call: JobCall, _ctx: &Ctx) -> Result<Self, Self::Rejection> {
        unimplemented!()
    }
}

impl A {
    #[debug_job]
    async fn job(&mut self) {}
}

fn main() {}
