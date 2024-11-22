use gadget_sdk::config::{GadgetConfiguration, StdGadgetConfiguration};
use gadget_sdk::ctx::{EVMProviderContext, KeystoreContext, ServicesContext, TangleClientContext};

#[derive(KeystoreContext, EVMProviderContext, TangleClientContext, ServicesContext)]
#[allow(dead_code)]
struct MyContext(String, #[config] StdGadgetConfiguration);

#[allow(dead_code)]
fn main() {
    let body = async {
        let ctx = MyContext("bar".to_string(), GadgetConfiguration::default());
        let _keystore = ctx.keystore();
        let _evm_provider = ctx.evm_provider().await;
        let tangle_client = ctx.tangle_client().await.unwrap();
        let _services = ctx.current_service_operators(&tangle_client).await;
    };
    drop(body);
}
