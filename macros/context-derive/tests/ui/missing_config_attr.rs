use gadget_sdk::config::StdGadgetConfiguration;
use gadget_sdk::ctx::KeystoreContext;

#[derive(KeystoreContext)]
struct MyContext {
    foo: String,
    sdk_config: StdGadgetConfiguration,
}

fn main() {}
