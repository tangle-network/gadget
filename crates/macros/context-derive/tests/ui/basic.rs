use async_trait::async_trait;
use blueprint_sdk::config::GadgetConfiguration;
use blueprint_sdk::contexts::instrumented_evm_client::EvmInstrumentedClientContext as _;
use blueprint_sdk::contexts::keystore::KeystoreContext as _;
use blueprint_sdk::contexts::services::ServicesContext as _;
use blueprint_sdk::contexts::tangle::TangleClientContext as _;
use blueprint_sdk::macros::ext::clients::GadgetServicesClient as _;
use blueprint_sdk::macros::ext::crypto::sp_core::SpEcdsa;
use blueprint_sdk::macros::ext::keystore::backends::Backend;
use blueprint_sdk::networking::networking::{Network, NetworkMultiplexer, ProtocolMessage};
use blueprint_sdk::networking::{GossipMsgKeyPair, GossipMsgPublicKey};
use blueprint_sdk::std::collections::BTreeMap;
use blueprint_sdk::std::sync::Arc;
use blueprint_sdk::stores::local_database::LocalDatabase;
use gadget_context_derive::{
    EVMProviderContext, KeystoreContext, ServicesContext, TangleClientContext,
};
use round_based::ProtocolMessage as RoundBasedProtocolMessage;
use serde::{Deserialize, Serialize};

#[derive(KeystoreContext, EVMProviderContext, TangleClientContext, ServicesContext)]
#[allow(dead_code)]
struct MyContext {
    foo: String,
    #[config]
    config: GadgetConfiguration,
    store: Arc<LocalDatabase<u64>>,
    #[call_id]
    call_id: Option<u64>,
}

#[allow(dead_code)]
fn main() {
    let body = async {
        let ctx = MyContext {
            foo: "bar".to_string(),
            config: GadgetConfiguration::default(),
            store: Arc::new(LocalDatabase::open("test.json")),
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

#[derive(RoundBasedProtocolMessage, Clone, Serialize, Deserialize)]
enum StubMessage {}

#[allow(dead_code)]
struct StubNetwork;

#[async_trait]
impl Network for StubNetwork {
    async fn next_message(&self) -> Option<ProtocolMessage> {
        None
    }

    async fn send_message(
        &self,
        message: ProtocolMessage,
    ) -> Result<(), blueprint_sdk::networking::error::Error> {
        drop(message);
        Ok(())
    }

    fn public_id(&self) -> GossipMsgPublicKey {
        todo!()
    }
}
