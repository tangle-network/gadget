use blueprint_sdk::runner::config::BlueprintEnvironment;
use gadget_context_derive::KeystoreContext;

#[derive(KeystoreContext)]
struct MyContext {
    foo: String,
    sdk_config: BlueprintEnvironment,
}

fn main() {}
