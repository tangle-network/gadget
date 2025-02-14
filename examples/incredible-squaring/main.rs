extern crate alloc;

use blueprint_sdk::*;
use futures_util::TryStreamExt;
use gadget_blueprint_serde::BoundedVec;
use gadget_blueprint_serde::Field;
use tangle_subxt::parity_scale_codec::Encode;
use tangle_subxt::subxt::utils::AccountId32;
use tower::filter::FilterLayer;
use tower::{Service, ServiceExt};

/// Tangle Network Integration
mod tangle;

use tangle::extract::{CallId, TangleArgs, TangleResult};

use tangle::filters::MatchesServiceId;
use tangle::producer::{TangleClient, TangleProducer};

use crate::tangle::filters::MismatchedServiceId;

// The job ID (to be generated?)
const XSQUARE_JOB_ID: u32 = 1;
const MULTIPLY_JOB_ID: u32 = 2;

// The context (any type that's `Clone` + `Send` + `Sync` + 'static)
#[derive(Clone, Debug)]
pub struct MyContext {
    foo: u64,
}

// The job function
//
// The arguments are made up of "extractors", which take a portion of the `JobCall` to convert into the
// target type.
//
// The context is passed in as a parameter, and can be used to store any shared state between job calls.
pub async fn square(
    CallId(call_id): CallId,
    Context(ctx): Context<MyContext>,
    TangleArgs(x): TangleArgs<u64>,
) -> impl IntoJobResult {
    println!("call_id: {}", call_id);
    println!("ctx.foo: {:?}", ctx.foo);
    println!("x: {}", x);
    let result = x * x;

    println!("result: {}", result);

    // The result is then converted into a `JobResult` to be sent back to the caller.
    TangleResult(result)
}

pub async fn multiply(
    CallId(call_id): CallId,
    Context(ctx): Context<MyContext>,
    TangleArgs((x, y)): TangleArgs<(u64, u64)>,
) -> impl IntoJobResult {
    println!("call_id: {}", call_id);
    println!("ctx.foo: {:?}", ctx.foo);
    println!("x: {}", x);
    println!("y: {}", y);
    let result = x * y;

    println!("result: {}", result);

    // The result is then converted into a `JobResult` to be sent back to the caller.
    TangleResult(result)
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // A router
    //
    // Each "route" is a job ID and the job function. We can also support arbitrary `Service`s from `tower`,
    // which may make it easier for people to port over existing services to a blueprint.
    let mut router = Router::new()
        .route(XSQUARE_JOB_ID, square)
        .route(MULTIPLY_JOB_ID, multiply)
        .with_context(MyContext { foo: 10 })
        .layer(FilterLayer::new(MatchesServiceId(1)));

    // Job calls will be created by the Producer
    let job_call = tangle::create_call()
        .job_id(XSQUARE_JOB_ID)
        .block_number(20)
        .call_id(42)
        .service_id(1)
        .args(Field::Uint64(2))
        .call();

    // `Router`s themselves are `Service`s, so we can convert it into one, and then call it with the `JobCall`.
    // The `Router` then finds the job function internally that matches the `job_id`, and calls it.
    let job_result = router.as_service().ready().await?.call(job_call).await?;

    // Proof that we got the right value
    assert_eq!(
        job_result.into_body(),
        Bytes::from(Field::<AccountId32>::Uint64(4).encode())
    );

    // Another job call example
    let job_call = tangle::create_call()
        .job_id(MULTIPLY_JOB_ID)
        .block_number(20)
        .call_id(43)
        .service_id(1)
        .args(Field::Array(BoundedVec(vec![
            Field::Uint64(2),
            Field::Uint64(3),
        ])))
        .call();

    let job_result = router.as_service().ready().await?.call(job_call).await?;

    assert_eq!(
        job_result.into_body(),
        Bytes::from(Field::<AccountId32>::Uint64(6).encode())
    );

    // A Job call with an different service ID
    let job_call = tangle::create_call()
        .job_id(MULTIPLY_JOB_ID)
        .block_number(20)
        .call_id(43)
        .service_id(2)
        .args(Field::Array(BoundedVec(vec![
            Field::Uint64(2),
            Field::Uint64(3),
        ])))
        .call();
    let job_result = router.as_service().ready().await?.call(job_call).await;
    assert!(job_result
        .unwrap_err()
        .downcast::<MismatchedServiceId>()
        .is_ok());

    let tangle_client = TangleClient::new().await?;
    let mut tangle_producer = TangleProducer::finalized_blocks(tangle_client).await?;

    // The `TangleProducer` is also a `Stream`, so we can use it to get job calls from the Tangle network.
    while let Some(job_call) = tangle_producer.try_next().await? {
        println!("job_call: {:?}", job_call);
        let job_result = router.as_service().ready().await?.call(job_call).await;
        match job_result {
            Ok(job_result) => {
                println!("job_result: {:?}", job_result);
            }
            // We can ignore mismatched service IDs
            Err(e) if e.is::<MismatchedServiceId>() => continue,
            Err(e) => {
                eprintln!("job_result error: {:?}", e);
            }
        }
    }

    Ok(())
}
