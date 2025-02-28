use blueprint_macros::debug_job;
use core::future::Future;

#[debug_job]
fn handler() -> impl Future<Output = ()> {
    async {}
}

fn main() {}
