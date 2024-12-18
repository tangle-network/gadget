use crate::error::SymbioticError;
use alloy_network::EthereumWallet;
use alloy_primitives::B256;
use gadget_config::{GadgetConfiguration, ProtocolSettings};
use gadget_runner_core::config::BlueprintConfig;
use gadget_runner_core::error::{RunnerError as Error, RunnerError};
use gadget_utils::gadget_utils_evm::{get_provider_http, get_wallet_provider_http};
use symbiotic_rs::OperatorRegistry;

#[derive(Clone, Copy, Default)]
pub struct SymbioticConfig {}

#[async_trait::async_trait]
impl BlueprintConfig for SymbioticConfig {
    async fn requires_registration(&self, env: &GadgetConfiguration) -> Result<bool, Error> {
        let contract_addresses = match env.protocol_settings {
            ProtocolSettings::Symbiotic(addresses) => addresses,
            _ => {
                return Err(gadget_runner_core::error::RunnerError::InvalidProtocol(
                    "Expected Symbiotic protocol".into(),
                ));
            }
        };
        let operator_registry_address = contract_addresses.operator_registry_address;

        // TODO: Get the address from GadgetConfiguration->Keystore->ECDSA->AlloyKey->Address
        // let operator_address = env.keystore()?.ecdsa_key()?.alloy_key()?.address();
        let operator_address =
            alloy_primitives::address!("0000000000000000000000000000000000000000");

        let operator_registry = OperatorRegistry::new(
            operator_registry_address,
            get_provider_http(&env.http_rpc_endpoint),
        );

        let is_registered = operator_registry
            .isEntity(operator_address)
            .call()
            .await
            .map(|r| r._0)
            .map_err(|e| {
                <SymbioticError as Into<RunnerError>>::into(SymbioticError::Registration(
                    e.to_string(),
                ))
            })?;

        Ok(!is_registered)
    }

    async fn register(&self, env: &GadgetConfiguration) -> Result<(), Error> {
        let contract_addresses = match env.protocol_settings {
            ProtocolSettings::Symbiotic(addresses) => addresses,
            _ => {
                return Err(gadget_runner_core::error::RunnerError::InvalidProtocol(
                    "Expected Symbiotic protocol".into(),
                ));
            }
        };
        let operator_registry_address = contract_addresses.operator_registry_address;

        // TODO: Get the Signer from GadgetConfiguration->Keystore->ECDSA->AlloyKey
        // let operator_signer = env.keystore()?.ecdsa_key()?.alloy_key()?;
        let operator_signer = alloy_signer_local::LocalSigner::from_bytes(&B256::new([1; 32]))
            .map_err(|e| {
                <SymbioticError as Into<RunnerError>>::into(SymbioticError::Registration(
                    e.to_string(),
                ))
            })?;

        let wallet = EthereumWallet::new(operator_signer);
        let provider = get_wallet_provider_http(&env.http_rpc_endpoint, wallet);
        let operator_registry = OperatorRegistry::new(operator_registry_address, provider.clone());

        let result = operator_registry
            .registerOperator()
            .send()
            .await
            .map_err(|e| {
                <SymbioticError as Into<RunnerError>>::into(SymbioticError::Registration(
                    e.to_string(),
                ))
            })?
            .get_receipt()
            .await
            .map_err(|e| {
                <SymbioticError as Into<RunnerError>>::into(SymbioticError::Registration(
                    e.to_string(),
                ))
            })?;

        if result.status() {
            gadget_logging::info!("Operator registered successfully");
        } else {
            gadget_logging::error!("Operator registration failed");
        }

        Ok(())
    }
}
