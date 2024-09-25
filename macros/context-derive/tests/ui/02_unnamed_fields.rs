use gadget_sdk::config::StdGadgetConfiguration;
use gadget_sdk::ctx::KeystoreContext;

#[derive(KeystoreContext)]
struct MyContext(String, #[config] StdGadgetConfiguration);

fn main() {
    let ctx = MyContext("bar".to_string(), Default::default());
    let _keystore = ctx.keystore();
}
