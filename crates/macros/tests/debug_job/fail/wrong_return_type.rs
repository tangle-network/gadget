use blueprint_macros::debug_job;

#[debug_job]
async fn job() -> bool {
    false
}

fn main() {}
