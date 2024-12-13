use std::collections::BTreeMap;
use subxt_core::tx::signer::Signer;
use subxt_core::utils::AccountId32;
use crate::contexts::{ServicesContext, TangleClientContext};

/// `MPCContext` trait provides access to MPC (Multi-Party Computation) functionality from the context.
#[async_trait::async_trait]
pub trait MPCContext: ServicesContext<Config = <Self as TangleClientContext>::Config> + TangleClientContext {
    /// Returns a reference to the configuration
    fn config(&self) -> &crate::config::StdGadgetConfiguration;

    /// Returns the network protocol identifier
    fn network_protocol(&self) -> String;

    /// Creates a network delivery wrapper for MPC communication
    fn create_network_delivery_wrapper<M>(
        &self,
        mux: std::sync::Arc<crate::network::NetworkMultiplexer>,
        party_index: crate::round_based::PartyIndex,
        task_hash: [u8; 32],
        parties: std::collections::BTreeMap<crate::round_based::PartyIndex, crate::subxt_core::ext::sp_core::ecdsa::Public>,
    ) -> Result<crate::network::round_based_compat::NetworkDeliveryWrapper<M>, crate::Error>
    where
        M: Clone + Send + Unpin + 'static + crate::serde::Serialize + crate::serde::de::DeserializeOwned + crate::round_based::ProtocolMessage,
    {
        Ok(crate::network::round_based_compat::NetworkDeliveryWrapper::new(mux, party_index, task_hash, parties))
    }

    /// Retrieves the current party index
    async fn get_party_index(
        &self,
    ) -> Result<crate::round_based::PartyIndex, crate::Error> {
        Ok(self.get_party_index_and_operators().await?.0 as _)
    }

    /// Retrieves the current participants in the MPC protocol
    async fn get_participants(
        &self,
    ) -> Result<
        std::collections::BTreeMap<crate::round_based::PartyIndex, crate::subxt::utils::AccountId32>,
        crate::Error,
    > {
        Ok(self.get_party_index_and_operators().await?.1.into_iter().enumerate().map(|(i, (id, _))| (i as _, id)).collect())
    }

    /// Retrieves the current blueprint ID from the configuration
    ///
    /// # Errors
    /// Returns an error if the blueprint ID is not found in the configuration
    fn blueprint_id(&self) -> crate::color_eyre::Result<u64> {
        self.config()
            .protocol_specific
            .tangle()
            .map(|c| c.blueprint_id)
            .map_err(|err| crate::color_eyre::Report::msg(format!("Blueprint ID not found in configuration: {err}")))
    }

    /// Retrieves the current call ID for this job
    ///
    /// # Errors
    /// Returns an error if failed to retrieve the call ID from storage
    #[deprecated(note = "Use `context.call_id` field instead. This function is highly prone to race conditions")]
    async fn current_call_id(&self) -> crate::color_eyre::Result<u64> {
        let client = self.tangle_client().await?;
        let addr = crate::ext::tangle_subxt::tangle_testnet_runtime::api::storage().services().next_job_call_id();
        let storage = client.storage().at_latest().await?;

        let maybe_call_id = storage
            .fetch_or_default(&addr)
            .await
            .map_err(|err| crate::color_eyre::Report::msg(format!("Failed to fetch current call ID: {err}")))?;

        Ok(maybe_call_id.saturating_sub(1))
    }

    /// Retrieves the ECDSA keys for all current service operators
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to connect to the Tangle client
    /// - Failed to retrieve operator information
    /// - Missing ECDSA key for any operator
    async fn current_service_operators_ecdsa_keys(
        &self,
    ) -> color_eyre::Result<BTreeMap<AccountId32, crate::subxt_core::ext::sp_core::ecdsa::Public>> {
        let client = self.tangle_client().await?;
        let current_blueprint = self.blueprint_id()?;
        let current_service_op = self.current_service_operators(&client).await?;
        let storage = client.storage().at_latest().await?;

        let mut map = std::collections::BTreeMap::new();
        for (operator, _) in current_service_op {
            let addr = crate::ext::tangle_subxt::tangle_testnet_runtime::api::storage()
                .services()
                .operators(current_blueprint, &operator);

            let maybe_pref = storage.fetch(&addr).await.map_err(|err| {
                crate::color_eyre::Report::msg(format!("Failed to fetch operator storage for {operator}: {err}"))
            })?;

            if let Some(pref) = maybe_pref {
                let _ = map.insert(operator, crate::subxt_core::ext::sp_core::ecdsa::Public(pref.key));
            } else {
                return Err(crate::color_eyre::Report::msg(format!("Missing ECDSA key for operator {operator}")));
            }
        }

        Ok(map)
    }

    /// Retrieves the current party index and operator mapping (ECDSA)
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to retrieve operator keys
    /// - Current party is not found in the operator list
    async fn get_party_index_and_operators(
        &self,
    ) -> crate::color_eyre::Result<(usize, std::collections::BTreeMap<crate::subxt::utils::AccountId32, crate::subxt_core::ext::sp_core::ecdsa::Public>)> {
        let parties = self.current_service_operators_ecdsa_keys().await?;
        let my_id = self.config().first_sr25519_signer()?.account_id();

        crate::trace!(
                    "Looking for {my_id:?} in parties: {:?}",
                    parties.keys().collect::<Vec<_>>()
                );

        let index_of_my_id = parties
            .iter()
            .position(|(id, _)| id == &my_id)
            .ok_or_else(|| crate::color_eyre::Report::msg("Party not found in operator list"))?;

        Ok((index_of_my_id, parties))
    }

    /// Retrieves the ECDSA keys for all current service operators (SR25519)
    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to connect to the Tangle client
    /// - Failed to retrieve operator information
    /// - Missing ECDSA key for any operator
    async fn current_service_operators_sr25519(
        &self,
    ) -> color_eyre::Result<BTreeMap<AccountId32, crate::subxt_core::ext::sp_core::sr25519::Public>> {
        let client = self.tangle_client().await?;
        let current_service_op = self.current_service_operators(&client).await?;

        let mut map = std::collections::BTreeMap::new();
        for (operator, _) in current_service_op {
            let _ = map.insert(operator.clone(), crate::subxt_core::ext::sp_core::sr25519::Public(operator.0));
        }

        Ok(map)
    }

    ///
    /// # Errors
    /// Returns an error if:
    /// - Failed to retrieve operator keys
    /// - Current party is not found in the operator list
    async fn get_party_index_and_operators_sr25519(
        &self,
    ) -> crate::color_eyre::Result<(usize, std::collections::BTreeMap<crate::subxt::utils::AccountId32, crate::subxt_core::ext::sp_core::sr25519::Public>)> {
        let parties = self.current_service_operators_sr25519().await?;
        let my_id = self.config().first_sr25519_signer()?.account_id();

        crate::trace!(
                    "Looking for {my_id:?} in parties: {:?}",
                    parties.keys().collect::<Vec<_>>()
                );

        let index_of_my_id = parties
            .iter()
            .position(|(id, _)| id == &my_id)
            .ok_or_else(|| crate::color_eyre::Report::msg("Party not found in operator list"))?;

        Ok((index_of_my_id, parties))
    }
}
