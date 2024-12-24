use gadget_config::{GadgetConfiguration, StdGadgetConfiguration};
use gadget_context_derive::{
    EVMProviderContext, KeystoreContext, ServicesContext, TangleClientContext,
};
use gadget_contexts::instrumented_evm_client::EvmInstrumentedClientContext as _;
use gadget_contexts::keystore::KeystoreContext as _;
use gadget_contexts::services::ServicesContext as _;
use gadget_contexts::tangle::TangleClientContext as _;

#[derive(KeystoreContext, EVMProviderContext, TangleClientContext, ServicesContext)]
#[allow(dead_code)]
struct MyContext(
    String,
    #[config] StdGadgetConfiguration,
    #[call_id] Option<u64>,
);

#[allow(dead_code)]
fn main() {
    let body = async {
        let ctx = MyContext("bar".to_string(), GadgetConfiguration::default(), None);
        let _keystore = ctx.keystore();
        let _evm_provider = ctx.evm_client().await;
        let _tangle_client = ctx.tangle_client().await.unwrap();
        let services_client = ctx.services_client().await;
        let _services = services_client.current_service_operators([0; 32], 0).await;
    };
    drop(body);
}
