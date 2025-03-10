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

impl<Ctx> FromJobCall<Ctx> for Box<A>
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
    async fn job(self) {}

    #[debug_job]
    async fn job_with_qualified_self(self: Box<Self>) {}
}

fn main() {}
