use crate::EmptyContext;
use gadget_blueprint_proc_macro::job;
use gadget_event_listeners::core::testing::PendingEventListener;
use std::convert::Infallible;

#[job(id = 0, event_listener(listener = PendingEventListener<u16, EmptyContext>), result(Vec<u8>, String))]
fn keygen(ctx: EmptyContext, n: u16) -> Result<(Vec<u8>, String), Infallible> {
    let _ = n;
    Ok((Vec::new(), String::new()))
}
