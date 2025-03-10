use blueprint_macros::debug_job;

#[debug_job(foo)]
async fn job() {}

fn main() {}
