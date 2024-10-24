use gadget_sdk::event_listener::tangle::{TangleEvent, TangleEventListener};
use gadget_sdk::job;

#[derive(Clone)]
pub struct MyContext;

#[job(
    id = 0,
    result(_),
    event_listener(
        listener = TangleEventListener<MyContext>,
    ),
)]
pub fn raw(event: TangleEvent<MyContext>, context: MyContext) -> Result<u64, gadget_sdk::Error> {
    Ok(0)
}
