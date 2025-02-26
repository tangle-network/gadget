extern crate alloc;

use async_trait::async_trait;
use blueprint_core::IntoJobResult;
use blueprint_core::{Context, Job};
use blueprint_router::Router;
use blueprint_runner::config::{GadgetConfiguration, TangleConfig};
use blueprint_runner::error::RunnerError;
use blueprint_runner::{BackgroundService, BlueprintRunner};
use blueprint_tangle_extra::consumer::TangleConsumer;
use blueprint_tangle_extra::extract::{
    BlockEvents, BlockNumber, CallId, Event, FirstEvent, LastEvent, TangleArg, TangleArgs2,
    TangleResult,
};
use blueprint_tangle_extra::filters::MatchesServiceId;
use blueprint_tangle_extra::layers::TangleLayer;
use blueprint_tangle_extra::producer::TangleProducer;
use gadget_core_testing_utils::harness::TestHarness;
use gadget_tangle_testing_utils::TangleTestHarness;
use tangle_subxt::tangle_testnet_runtime::api::balances::events::Transfer;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;
use tower::filter::FilterLayer;
use tracing::error;
use tracing_subscriber::filter::LevelFilter;

// The job ID (to be generated?)
const XSQUARE_JOB_ID: u32 = 0;
const MULTIPLY_JOB_ID: u32 = 1;

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
    TangleArg(x): TangleArg<u64>,
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
    TangleArgs2(x, y): TangleArgs2<u64, u64>,
) -> impl IntoJobResult {
    println!("call_id: {}", call_id);
    println!("ctx.foo: {:?}", ctx.foo);
    println!("x: {}", x);
    println!("y: {}", y);
    let result = x * y;

    tracing::info!("result: {}", result);

    // The result is then converted into a `JobResult` to be sent back to the caller.
    TangleResult(result)
}

/// An Example of a job function that uses the `Event` extractors
/// NOTE: this job will fail if no Transfer events are found in the block
/// to avoid this, you can use the `Option<Event<T>>` extractor which
/// will return `None` if no events are found.
pub async fn on_transfer(
    BlockNumber(block_number): BlockNumber,
    // The `Event` extractor will extract all the `Transfer` events that happened in the current block.
    Event(transfers): Event<Transfer>,
    // Or maybe you are interested in the first transfer event
    FirstEvent(first_transfer): FirstEvent<Transfer>,
    // you can also get the last transfer event
    LastEvent(last_transfer): LastEvent<Transfer>,
) -> impl IntoJobResult {
    println!("on_transfer");
    println!("Block number: {}", block_number);

    for transfer in transfers {
        println!("Transfer: {:?}", transfer);
    }

    println!("First Transfer: {:?}", first_transfer);
    println!("Last Transfer: {:?}", last_transfer);
}

/// An Example of a job function that uses the `BlockEvents` extractor
/// This should be used when you want to manually handle the events
pub async fn manual_event_handling(
    BlockNumber(block_number): BlockNumber,
    // This allows you to extract all the events that happened in the current block
    BlockEvents(events): BlockEvents,
) -> impl IntoJobResult {
    println!("Manual Event Handling for Block Number: {}", block_number);

    for event in events.iter() {
        let details = event.unwrap();
        println!("Event: {:?}", details.variant_name());
    }
}

#[derive(Clone)]
struct FooBackgroundService;

#[async_trait]
impl BackgroundService for FooBackgroundService {
    async fn start(&self) -> Result<Receiver<Result<(), RunnerError>>, RunnerError> {
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let _ = tx.send(Ok(()));
        });
        Ok(rx)
    }
}

#[tokio::main]
async fn main() -> Result<(), blueprint_core::BoxError> {
    setup_log();

    // Sets up the tangle node for the producer
    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;

    tracing::info!(
        "Tangle node running on port: {}",
        harness.http_endpoint.port().unwrap()
    );

    let tangle_client = harness.client().subxt_client().clone();
    let tangle_config = TangleConfig::default();

    let (_test_env, service_id, _blueprint_id) = harness.setup_services::<1>(false).await?;

    let tangle_producer = TangleProducer::finalized_blocks(tangle_client.clone()).await?;
    let tangle_consumer = TangleConsumer::new(tangle_client, harness.sr25519_signer.clone());

    let result = BlueprintRunner::new(tangle_config, GadgetConfiguration::default())
        .router(
            // A router
            //
            // Each "route" is a job ID and the job function. We can also support arbitrary `Service`s from `tower`,
            // which may make it easier for people to port over existing services to a blueprint.
            Router::new()
                // The two routes defined here have the `TangleLayer`, which adds metadata to the
                // produced `JobResult`s, making it visible to a `TangleConsumer`.
                .route(XSQUARE_JOB_ID, square.layer(TangleLayer))
                .route(MULTIPLY_JOB_ID, multiply.layer(TangleLayer))
                // Jobs that "always" run, regardless of the job ID. These will be called even if the job ID
                // matches another router.
                .always(on_transfer)
                .always(manual_event_handling)
                // We can add a context to the router, which will be passed to all job functions
                // that have the `Context` extractor.
                // TODO: This means a *lot* of cloning, need to inform users that their context
                //       should be cheaply cloneable.
                .with_context(MyContext { foo: 10 })
                // Add the `FilterLayer` to filter out job calls that don't match the service ID
                .layer(FilterLayer::new(MatchesServiceId(service_id))),
        )
        .background_service(FooBackgroundService)
        // Add potentially many producers
        //
        // A producer is simply a `Stream` that outputs `JobCall`s, which are passed down to the intended
        // job functions.
        .producer(tangle_producer)
        // Add potentially many consumers
        //
        // A consumer is simply a `Sink` that consumes `JobResult`s, which are the output of the job functions.
        // Every result will be passed to every consumer. It is the responsibility of the consumer
        // to determine whether or not to process a result.
        .consumer(tangle_consumer)
        // Custom shutdown handlers
        //
        // Now users can specify what to do when an error occurs and the runner is shutting down.
        // That can be cleanup logic, finalizing database transactions, etc.
        .with_shutdown_handler(async { println!("Shutting down!") })
        .run()
        .await;

    if let Err(e) = result {
        error!("Runner failed! {e:?}");
    }

    Ok(())
}

pub fn setup_log() {
    use tracing_subscriber::util::SubscriberInitExt;

    let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
        .without_time()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .finish()
        .try_init();
}
