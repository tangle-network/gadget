use crate::EmptyContext;
use blueprint_sdk::event_listeners::core::testing::PendingEventListener;
use blueprint_sdk::macros::job;

#[job(id = 0, event_listener(listener = PendingEventListener<u16, EmptyContext>), result(_))]
#[allow(clippy::unused_unit)]
fn keygen(ctx: EmptyContext, n: u16) -> () {}
