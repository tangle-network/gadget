use gadget_sdk::config::StdGadgetConfiguration;
use gadget_sdk::ctx::KeystoreContext;

#[derive(KeystoreContext)]
struct MyContext {
    foo: String,
    #[config]
    sdk_config: StdGadgetConfiguration,
}

fn main() {
    let ctx = MyContext {
        foo: "bar".to_string(),
        sdk_config: Default::default(),
    };
    let _keystore = ctx.keystore();
}
