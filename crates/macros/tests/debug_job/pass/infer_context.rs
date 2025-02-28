use blueprint_macros::debug_job;
use blueprint_sdk::Context;

#[debug_job]
async fn job(_: Context<AppContext>) {}

#[debug_job]
async fn job_2(_: blueprint_sdk::Context<AppContext>) {}

#[debug_job]
async fn job_3(_: blueprint_sdk::Context<AppContext>, _: blueprint_sdk::Context<AppContext>) {}

#[debug_job]
async fn job_4(_: Context<AppContext>, _: Context<AppContext>) {}

#[debug_job]
async fn job_5(_: blueprint_sdk::Context<AppContext>, _: Context<AppContext>) {}

#[derive(Clone)]
struct AppContext;

fn main() {}
