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
struct MyContext<T: Send + Sync, U: Send + Sync> {
    foo: T,
    bar: U,
    #[config]
    sdk_config: BlueprintEnvironment,
    #[call_id]
    call_id: Option<u64>,
}

#[allow(dead_code)]
fn main() {
    let body = async {
        let ctx = MyContext {
            foo: "bar".to_string(),
            bar: 42,
            sdk_config: BlueprintEnvironment::default(),
            call_id: None,
        };
        let _keystore = ctx.keystore();
        let _evm_provider = ctx.evm_client();
        let _tangle_client = ctx.tangle_client().await.unwrap();
        let services_client = ctx.services_client().await;
        let _services = services_client
            .current_service_operators([0; 32], 0)
            .await
            .unwrap();
    };

    drop(body);
}
