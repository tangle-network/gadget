use crate::EmptyContext;
use gadget_blueprint_proc_macro::job;
use gadget_event_listeners::core::testing::PendingEventListener;
use std::convert::Infallible;

#[job(id = 0, event_listener(listener = PendingEventListener<(u16, String), EmptyContext>))]
fn keygen(ctx: EmptyContext, args: (u16, String)) -> Result<Vec<u8>, Infallible> {
    let _ = args;
    Ok(Vec::new())
}
