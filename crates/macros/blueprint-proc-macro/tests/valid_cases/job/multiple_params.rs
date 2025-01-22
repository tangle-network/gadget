use crate::EmptyContext;
use blueprint_sdk::event_listeners::core::testing::PendingEventListener;
use blueprint_sdk::macros::job;
use blueprint_sdk::std::convert::Infallible;

#[job(id = 0, event_listener(listener = PendingEventListener<(u16, u8), EmptyContext>), result(Vec<u8>))]
fn keygen(ctx: EmptyContext, args: (u16, u8)) -> Result<Vec<u8>, Infallible> {
    let _ = args;
    Ok(Vec::new())
}
