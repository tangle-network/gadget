use crate::EmptyContext;
use blueprint_sdk::event_listeners::core::testing::PendingEventListener;
use blueprint_sdk::macros::job;

#[job(id = 0, event_listener(listener = PendingEventListener<u16, EmptyContext>), result(Vec<u8>))]
fn keygen(ctx: EmptyContext, n: u16) -> Vec<u8> {
    Vec::new()
}
