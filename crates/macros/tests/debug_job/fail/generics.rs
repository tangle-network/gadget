use blueprint_macros::debug_job;

#[debug_job]
async fn job<T>(_extract: T) {}

fn main() {}
