use crate::EmptyContext;
use gadget_blueprint_proc_macro::job;
use gadget_sdk::event_listener::testing::PendingEventListener;

#[job(id = 0, event_listener(listener = PendingEventListener<u16, EmptyContext>), result(Vec<u8>))]
fn keygen(ctx: EmptyContext, n: u16) -> Vec<u8> {
    Vec::new()
}
