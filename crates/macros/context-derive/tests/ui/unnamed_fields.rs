use blueprint_sdk::contexts::instrumented_evm_client::EvmInstrumentedClientContext as _;
use blueprint_sdk::contexts::keystore::KeystoreContext as _;
use blueprint_sdk::contexts::services::ServicesContext as _;
use blueprint_sdk::contexts::tangle::TangleClientContext as _;
use blueprint_sdk::runner::config::BlueprintEnvironment;
use gadget_context_derive::{
    EVMProviderContext, KeystoreContext, ServicesContext, TangleClientContext,
};

#[derive(KeystoreContext, EVMProviderContext, TangleClientContext, ServicesContext)]
#[allow(dead_code)]
struct MyContext(
    String,
    #[config] BlueprintEnvironment,
    #[call_id] Option<u64>,
);

#[allow(dead_code)]
fn main() {
    let body = async {
        let ctx = MyContext("bar".to_string(), BlueprintEnvironment::default(), None);
        let _keystore = ctx.keystore();
        let _evm_provider = ctx.evm_client();
        let tangle_client = ctx.tangle_client().await.unwrap();
        let _services_client = ctx.services_client().await;
        let _services = tangle_client
            .services_client()
            .current_service_operators([0; 32], 0)
            .await;
    };
    drop(body);
}
