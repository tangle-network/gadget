use blueprint_sdk::job::*;
use blueprint_sdk::*;
use tangle::CallId;
use tower::{Service, ServiceExt};

/// Tangle Network Integration
mod tangle;

use tangle::Args;

// The job ID (to be generated?)
const XSQUARE_JOB_ID: u32 = 1;

// The context (any type that's `Clone` + `Send` + `Sync` + 'static)
#[derive(Clone, Debug)]
pub struct MyContext {
    foo: u64,
}

const RETURN_VALUE: [u8; 3] = [1, 2, 3];

// The job function
//
// The arguments are made up of "extractors", which take a portion of the `JobCall` to convert into the
// target type.
pub async fn square(
    CallId(call_id): CallId,
    Context(ctx): Context<MyContext>,
) -> impl IntoJobResult {
    println!("call_id: {}", call_id);
    println!("ctx: {:?}", ctx);
    RETURN_VALUE
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // A router
    //
    // Each "route" is a job ID and the job function. We can also support arbitrary `Service`s from `tower`,
    // which may make it easier for people to port over existing services to a blueprint.
    let mut router: Router<()> = Router::new()
        .route(XSQUARE_JOB_ID, square)
        .with_context(MyContext { foo: 10 });

    // Job calls will be created by the Producer
    let job_call = tangle::create_call()
        .job_id(XSQUARE_JOB_ID)
        .call_id(42)
        .call();

    // `Router`s themselves are `Service`s, so we can convert it into one, and then call it with the `JobCall`.
    // The `Router` then finds the job function internally that matches the `job_id`, and calls it.
    let job_result = router
        .as_service::<Bytes>()
        .ready()
        .await?
        .call(job_call)
        .await?;

    // Proof that we got the right value
    assert_eq!(job_result.into_body(), Bytes::from(RETURN_VALUE.as_slice()));
    Ok(())
}
