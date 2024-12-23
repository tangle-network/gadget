use crate::EmptyContext;
use gadget_blueprint_proc_macro::job;
use gadget_sdk::event_listener::testing::PendingEventListener;

#[job(id = 0, event_listener(listener = PendingEventListener<u16, EmptyContext>), result(_))]
#[allow(clippy::unused_unit)]
fn keygen(ctx: EmptyContext, n: u16) -> [u8; 2] {
    [0, 1]
}
