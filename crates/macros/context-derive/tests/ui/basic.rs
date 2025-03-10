use blueprint_sdk::clients::GadgetServicesClient as _;
use blueprint_sdk::contexts::instrumented_evm_client::EvmInstrumentedClientContext as _;
use blueprint_sdk::contexts::keystore::KeystoreContext as _;
use blueprint_sdk::contexts::services::ServicesContext as _;
use blueprint_sdk::contexts::tangle::TangleClientContext as _;
use blueprint_sdk::runner::config::BlueprintEnvironment;
use blueprint_sdk::std::sync::Arc;
use blueprint_sdk::stores::local_database::LocalDatabase;
use gadget_context_derive::{
    EVMProviderContext, KeystoreContext, ServicesContext, TangleClientContext,
};

#[derive(KeystoreContext, EVMProviderContext, TangleClientContext, ServicesContext)]
#[allow(dead_code)]
struct MyContext {
    foo: String,
    #[config]
    config: BlueprintEnvironment,
    store: Arc<LocalDatabase<u64>>,
    #[call_id]
    call_id: Option<u64>,
}

#[allow(dead_code)]
fn main() {
    let body = async {
        let ctx = MyContext {
            foo: "bar".to_string(),
            config: BlueprintEnvironment::default(),
            store: Arc::new(LocalDatabase::open("test.json").unwrap()),
            call_id: None,
        };

        // Test existing context functions
        let _keystore = ctx.keystore();
        let _evm_provider = ctx.evm_client();
        let tangle_client = ctx.tangle_client().await.unwrap();
        let _services_client = ctx.services_client().await;
        let _services = tangle_client
            .services_client()
            .current_service_operators([0; 32], 0)
            .await
            .unwrap();

        // Test blueprint ID retrieval
        let _blueprint_id = tangle_client.blueprint_id();

        // Test party index and operators retrieval
        let _party_idx_ops = tangle_client.get_party_index_and_operators().await;
    };

    drop(body);
}
