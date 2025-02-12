use blueprint_sdk::job::*;
use blueprint_sdk::*;
use tangle::CallId;
use tower::{Service, ServiceExt};

/// Tangle Network Integration
mod tangle;

use tangle::Args;

const XSQUARE_JOB_ID: u32 = 1;

#[derive(Clone, Debug)]
pub struct MyContext;

pub struct SquareArgs {
    pub x: u64,
}

pub async fn square(
    CallId(call_id): CallId,
    Context(ctx): Context<MyContext>,
    Args(SquareArgs { x }): Args<SquareArgs>,
) -> impl IntoJobResult {
    println!("call_id: {}", call_id);
    println!("ctx: {:?}", ctx);
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let mut router = Router::new()
        .with_context(MyContext)
        .route(XSQUARE_JOB_ID, square);

    let job_call = tangle::create_call()
        .job_id(XSQUARE_JOB_ID)
        .call_id(42)
        .call();

    let job_result = router
        .as_service::<Bytes>()
        .ready()
        .await?
        .call(job_call)
        .await?;

    assert_eq!(job_result.into_body(), Bytes::new());
    Ok(())
}
