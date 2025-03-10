use blueprint_macros::debug_job;
use blueprint_sdk::{FromJobCall, JobCall, extract::FromRef};

#[debug_job(context = AppContext)]
async fn handler(_: A) {}

#[derive(Clone)]
struct AppContext;

struct A;

impl<Ctx> FromJobCall<Ctx> for A
where
    Ctx: Send + Sync,
    AppContext: FromRef<Ctx>,
{
    type Rejection = ();

    async fn from_job_call(_call: JobCall, _ctx: &Ctx) -> Result<Self, Self::Rejection> {
        unimplemented!()
    }
}

fn main() {}
