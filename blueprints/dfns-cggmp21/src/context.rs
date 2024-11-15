use cggmp21::security_level::SecurityLevel128;
use cggmp21::supported_curves::Secp256k1;
use cggmp21::KeyShare;
use color_eyre::eyre;
use gadget_sdk as sdk;
use gadget_sdk::network::{NetworkMultiplexer, StreamKey};
use gadget_sdk::store::LocalDatabase;
use gadget_sdk::subxt_core::ext::sp_core::{ecdsa, keccak_256};
use gadget_sdk::subxt_core::utils::AccountId32;
use key_share::CoreKeyShare;
use sdk::ctx::{KeystoreContext, ServicesContext, TangleClientContext};
use sdk::tangle_subxt::tangle_testnet_runtime::api;
use serde::{Deserialize, Serialize};
use sp_core::ecdsa::Public;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// The network protocol for the DFNS service
const NETWORK_PROTOCOL: &str = "/dfns/cggmp21/1.0.0";

/// DFNS-CGGMP21 Service Context that holds all the necessary context for the service
/// to run
#[derive(Clone, KeystoreContext, TangleClientContext, ServicesContext)]
pub struct DfnsContext {
    /// The overreaching configuration for the service
    #[config]
    pub config: sdk::config::StdGadgetConfiguration,
    /// The gossip handle for the network
    pub network_backend: Arc<NetworkMultiplexer>,
    /// The key-value store for the service
    pub store: Arc<sdk::store::LocalDatabase<DfnsStore>>,
    /// Identity
    pub identity: ecdsa::Pair,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DfnsStore {
    pub inner: Option<CoreKeyShare<Secp256k1>>,
    pub refreshed_key: Option<KeyShare<Secp256k1, SecurityLevel128>>,
}

impl DfnsContext {
    /// Create a new service context
    pub fn new(config: sdk::config::StdGadgetConfiguration) -> eyre::Result<Self> {
        let network_config = config.libp2p_network_config(NETWORK_PROTOCOL)?;
        let identity = network_config.ecdsa_key.clone();
        let gossip_handle = sdk::network::setup::start_p2p_network(network_config)
            .map_err(|e| eyre::eyre!("Failed to start the network: {e:?}"))?;
        let keystore_dir = PathBuf::from(config.keystore_uri.clone()).join("dfns.json");

        Ok(Self {
            store: Arc::new(LocalDatabase::open(keystore_dir)),
            identity,
            config,
            network_backend: Arc::new(NetworkMultiplexer::new(gossip_handle)),
        })
    }

    /// Get the key-value store
    pub fn store(&self) -> Arc<LocalDatabase<DfnsStore>> {
        self.store.clone()
    }

    /// Get the configuration
    pub fn config(&self) -> &sdk::config::StdGadgetConfiguration {
        &self.config
    }

    /// Get the network protocol
    pub fn network_protocol(&self) -> &str {
        NETWORK_PROTOCOL
    }

    /// Get the current blueprint id
    pub fn blueprint_id(&self) -> eyre::Result<u64> {
        self.config()
            .protocol_specific
            .tangle()
            .map(|c| c.blueprint_id)
            .map_err(|e| eyre::eyre!("Failed to get blueprint id: {e}"))
    }

    pub async fn get_party_index_and_operators(
        &self,
    ) -> eyre::Result<(usize, BTreeMap<AccountId32, Public>)> {
        let parties = self.current_service_operators_ecdsa_keys().await?;
        let ecdsa_id = self.config.first_ecdsa_signer()?.into_inner();
        let my_id = ecdsa_id.account_id();
        let index_of_my_id = parties
            .iter()
            .position(|(id, _)| id == my_id)
            .ok_or_else(|| eyre::eyre!("Failed to get party index"))?;

        Ok((index_of_my_id, parties))
    }

    /// Get Current Service Operators' ECDSA Keys as a map.
    pub async fn current_service_operators_ecdsa_keys(
        &self,
    ) -> eyre::Result<BTreeMap<AccountId32, ecdsa::Public>> {
        let client = self.tangle_client().await?;
        let current_blueprint = self.blueprint_id()?;
        let current_service_op = self.current_service_operators(&client).await?;
        let storage = client.storage().at_latest().await?;
        let mut map = BTreeMap::new();
        for (operator, _) in current_service_op {
            let addr = api::storage()
                .services()
                .operators(current_blueprint, &operator);
            let maybe_pref = storage.fetch(&addr).await?;
            if let Some(pref) = maybe_pref {
                map.insert(operator, ecdsa::Public(pref.key));
            } else {
                return Err(eyre::eyre!(
                    "Failed to get operator's {operator} public ecdsa key"
                ));
            }
        }

        Ok(map)
    }

    /// Get the current call id for this job.
    pub async fn current_call_id(&self) -> Result<u64, eyre::Error> {
        let client = self.tangle_client().await?;
        let addr = api::storage().services().next_job_call_id();
        let storage = client.storage().at_latest().await?;
        let maybe_call_id = storage.fetch_or_default(&addr).await?;
        Ok(maybe_call_id.saturating_sub(1))
    }

    /// Get the network backend for keygen job
    pub fn keygen_network_backend(&self, call_id: u64) -> impl sdk::network::Network {
        self.network_backend.multiplex(StreamKey {
            task_hash: keccak_256(&[&b"keygen"[..], &call_id.to_le_bytes()[..]].concat()),
            round_id: -1,
        })
    }

    /// Get the network backend for signing job
    pub fn signing_network_backend(&self, call_id: u64) -> impl sdk::network::Network {
        self.network_backend.multiplex(StreamKey {
            task_hash: keccak_256(&[&b"signing"[..], &call_id.to_le_bytes()[..]].concat()),
            round_id: -1,
        })
    }
}
