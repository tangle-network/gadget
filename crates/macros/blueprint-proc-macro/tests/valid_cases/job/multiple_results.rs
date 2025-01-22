use crate::EmptyContext;
use blueprint_sdk::event_listeners::core::testing::PendingEventListener;
use blueprint_sdk::macros::job;
use blueprint_sdk::std::convert::Infallible;

#[job(id = 0, event_listener(listener = PendingEventListener<u16, EmptyContext>), result(Vec<u8>, String))]
fn keygen(ctx: EmptyContext, n: u16) -> Result<(Vec<u8>, String), Infallible> {
    let _ = n;
    Ok((Vec::new(), String::new()))
}
