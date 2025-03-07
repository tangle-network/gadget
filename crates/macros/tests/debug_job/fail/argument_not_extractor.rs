use blueprint_macros::debug_job;

#[debug_job]
async fn job(_foo: bool) {}

fn main() {}
