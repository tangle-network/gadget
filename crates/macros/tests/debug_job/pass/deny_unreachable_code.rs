#![deny(unreachable_code)]

use blueprint_sdk::JobCall;

#[blueprint_macros::debug_job]
async fn job(_call: JobCall) {}

fn main() {}
