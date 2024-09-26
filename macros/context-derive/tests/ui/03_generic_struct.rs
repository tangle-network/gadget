use gadget_sdk::config::StdGadgetConfiguration;
use gadget_sdk::ctx::KeystoreContext;

#[derive(KeystoreContext)]
struct MyContext<T, U> {
    foo: T,
    bar: U,
    #[config]
    sdk_config: StdGadgetConfiguration,
}

fn main() {
    let ctx = MyContext {
        foo: "bar".to_string(),
        bar: 42,
        sdk_config: Default::default(),
    };
    let _keystore = ctx.keystore();
}
