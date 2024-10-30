use crate::{
    config::{GadgetConfiguration, ProtocolSpecificSettings},
    error,
    events_watcher::evm::{get_provider_http, get_wallet_provider_http},
    info,
    keystore::BackendExt,
};
use alloy_network::EthereumWallet;
use symbiotic_rs::OperatorRegistry;

use super::{BlueprintConfig, RunnerError};

#[derive(Clone, Copy)]
pub struct SymbioticConfig {}

#[async_trait::async_trait]
impl BlueprintConfig for SymbioticConfig {
    async fn requires_registration(
        &self,
        env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<bool, RunnerError> {
        let ProtocolSpecificSettings::Symbiotic(contract_addresses) = &env.protocol_specific else {
            return Err(RunnerError::InvalidProtocol(
                "Expected Symbiotic protocol".into(),
            ));
        };
        let operator_registry_address = contract_addresses.operator_registry_address;

        let operator_address = env.keystore()?.ecdsa_key()?.alloy_key()?.address();
        let operator_registry = OperatorRegistry::new(
            operator_registry_address,
            get_provider_http(&env.http_rpc_endpoint),
        );

        let is_registered = operator_registry
            .isEntity(operator_address)
            .call()
            .await
            .map(|r| r._0)
            .map_err(|e| RunnerError::SymbioticError(e.to_string()))?;

        Ok(!is_registered)
    }

    async fn register(
        &self,
        env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<(), RunnerError> {
        let ProtocolSpecificSettings::Symbiotic(contract_addresses) = &env.protocol_specific else {
            return Err(RunnerError::InvalidProtocol(
                "Expected Symbiotic protocol".into(),
            ));
        };
        let operator_registry_address = contract_addresses.operator_registry_address;

        let operator_signer = env.keystore()?.ecdsa_key()?.alloy_key()?;
        let wallet = EthereumWallet::new(operator_signer);
        let provider = get_wallet_provider_http(&env.http_rpc_endpoint, wallet);
        let operator_registry = OperatorRegistry::new(operator_registry_address, provider.clone());

        let result = operator_registry
            .registerOperator()
            .send()
            .await?
            .get_receipt()
            .await?;

        if result.status() {
            info!("Operator registered successfully");
        } else {
            error!("Operator registration failed");
        }

        Ok(())
    }
}
