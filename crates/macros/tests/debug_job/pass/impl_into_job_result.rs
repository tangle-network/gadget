use blueprint_macros::debug_job;
use blueprint_sdk::IntoJobResult;

#[debug_job]
async fn job() -> impl IntoJobResult {
    "hi!"
}

fn main() {}
