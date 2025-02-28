use blueprint_sdk::{Context, Router, extract::FromRef};

// This will implement `FromRef` for each field in the struct.
#[derive(Clone, FromRef)]
struct AppContext {
    auth_token: String,
}

// So those types can be extracted via `Context`
async fn job(_: Context<String>) {}

fn main() {
    let ctx = AppContext {
        auth_token: Default::default(),
    };

    let _: Router = Router::new().route(0, job).with_context(ctx);
}
