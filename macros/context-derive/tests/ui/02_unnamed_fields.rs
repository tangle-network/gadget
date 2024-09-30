use gadget_sdk::config::StdGadgetConfiguration;
use gadget_sdk::ctx::{EVMProviderContext, KeystoreContext, ServicesContext, TangleClientContext};

#[derive(KeystoreContext, EVMProviderContext, TangleClientContext, ServicesContext)]
struct MyContext(String, #[config] StdGadgetConfiguration);

fn main() {
    let body = async {
        let ctx = MyContext("bar".to_string(), Default::default());
        let _keystore = ctx.keystore();
        let _evm_provider = ctx.evm_provider().await;
        let tangle_client = ctx.tangle_client().await.unwrap();
        let _services = ctx.current_service_operators(&tangle_client).await;
    };
    // Run the async block
    let _ = body;
}
