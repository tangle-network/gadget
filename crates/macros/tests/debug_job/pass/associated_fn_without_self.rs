use blueprint_macros::debug_job;

struct A;

impl A {
    #[debug_job]
    async fn job() {}
}

fn main() {}
