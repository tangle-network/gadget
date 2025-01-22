use blueprint_sdk::config::GadgetConfiguration;
use gadget_context_derive::KeystoreContext;

#[derive(KeystoreContext)]
struct MyContext {
    foo: String,
    sdk_config: GadgetConfiguration,
}

fn main() {}
