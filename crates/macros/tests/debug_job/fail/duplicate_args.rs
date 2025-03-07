use blueprint_macros::debug_job;

#[debug_job(context = (), context = ())]
async fn job() {}

fn main() {}
