use gadget_sdk::config::StdGadgetConfiguration;
use gadget_sdk::ctx::{EVMProviderContext, KeystoreContext};

#[derive(KeystoreContext, EVMProviderContext)]
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
    let _evm_provider = ctx.evm_provider();
}
