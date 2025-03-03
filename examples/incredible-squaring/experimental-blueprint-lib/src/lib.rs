use blueprint_core::Context;
use blueprint_core::IntoJobResult;
use blueprint_runner::BackgroundService;
use blueprint_runner::error::RunnerError;
use blueprint_tangle_extra::extract::{
    BlockEvents, BlockNumber, CallId, Event, FirstEvent, LastEvent, TangleArg, TangleArgs2,
    TangleResult,
};
use tangle_subxt::tangle_testnet_runtime::api::balances::events::Transfer;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;

// The job ID (to be generated?)
pub const XSQUARE_JOB_ID: u32 = 0;
pub const MULTIPLY_JOB_ID: u32 = 1;

// The context (any type that's `Clone` + `Send` + `Sync` + 'static)
#[derive(Clone, Debug)]
pub struct MyContext {
    pub foo: u64,
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
) -> TangleResult<u64> {
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

    println!("result: {}", result);

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
pub struct FooBackgroundService;

impl BackgroundService for FooBackgroundService {
    async fn start(&self) -> Result<Receiver<Result<(), RunnerError>>, RunnerError> {
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let _ = tx.send(Ok(()));
        });
        Ok(rx)
    }
}
