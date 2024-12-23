use crate::EmptyContext;
use gadget_blueprint_proc_macro::job;
use gadget_event_listeners::core::testing::PendingEventListener;

#[job(id = 0, event_listener(listener = PendingEventListener<u16, EmptyContext>), result(_))]
#[allow(clippy::unused_unit)]
fn keygen(ctx: EmptyContext, n: u16) -> () {}
