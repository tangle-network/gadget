use gadget_config::GadgetConfiguration;
use gadget_context_derive::KeystoreContext;

#[derive(KeystoreContext)]
struct MyContext {
    foo: String,
    sdk_config: GadgetConfiguration,
}

fn main() {}
