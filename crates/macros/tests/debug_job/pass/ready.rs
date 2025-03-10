use blueprint_macros::debug_job;
use std::future::{Ready, ready};

#[debug_job]
fn job() -> Ready<()> {
    ready(())
}

fn main() {}
