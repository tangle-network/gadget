use blueprint_core::Job;
use blueprint_router::Router;
use blueprint_runner::BlueprintRunner;
use blueprint_runner::config::{GadgetConfiguration, TangleConfig};
use blueprint_tangle_extra::consumer::TangleConsumer;
use blueprint_tangle_extra::filters::MatchesServiceId;
use blueprint_tangle_extra::layers::TangleLayer;
use blueprint_tangle_extra::producer::TangleProducer;
use experimental_blueprint_lib::{
    FooBackgroundService, MULTIPLY_JOB_ID, MyContext, XSQUARE_JOB_ID, manual_event_handling,
    multiply, on_transfer, square,
};
use gadget_core_testing_utils::harness::TestHarness;
use gadget_tangle_testing_utils::harness::TangleTestHarness;
use tower::filter::FilterLayer;
use tracing::error;
use tracing::level_filters::LevelFilter;

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

    let result = BlueprintRunner::builder(tangle_config, GadgetConfiguration::default())
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
