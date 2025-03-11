use crate::BlueprintConfig;
use crate::config::BlueprintEnvironment;
use crate::error::RunnerError;
use alloy_primitives::{Address, FixedBytes, U256, hex};
use alloy_signer::Signer;
use alloy_signer_local::PrivateKeySigner;
use blueprint_evm_extra::util::get_provider_http;
use eigensdk::client_elcontracts::{reader::ELChainReader, writer::ELChainWriter};
use eigensdk::logging::get_test_logger;
use eigensdk::types::operator::Operator;
use eigensdk::utils::rewardsv2::middleware::ecdsastakeregistry::ECDSAStakeRegistry;
use eigensdk::utils::rewardsv2::middleware::ecdsastakeregistry::ISignatureUtils::SignatureWithSaltAndExpiry;
use gadget_keystore::backends::Backend;
use gadget_keystore::backends::eigenlayer::EigenlayerBackend;
use gadget_keystore::crypto::k256::K256Ecdsa;
use std::str::FromStr;

#[derive(Clone, Copy)]
pub struct EigenlayerECDSAConfig {
    earnings_receiver_address: Address,
    delegation_approver_address: Address,
}

impl EigenlayerECDSAConfig {
    #[must_use]
    pub fn new(earnings_receiver_address: Address, delegation_approver_address: Address) -> Self {
        Self {
            earnings_receiver_address,
            delegation_approver_address,
        }
    }
}

impl BlueprintConfig for EigenlayerECDSAConfig {
    async fn register(&self, env: &BlueprintEnvironment) -> Result<(), RunnerError> {
        register_ecdsa_impl(
            env,
            self.earnings_receiver_address,
            self.delegation_approver_address,
        )
        .await
    }

    async fn requires_registration(&self, env: &BlueprintEnvironment) -> Result<bool, RunnerError> {
        requires_registration_ecdsa_impl(env).await
    }
}

async fn requires_registration_ecdsa_impl(env: &BlueprintEnvironment) -> Result<bool, RunnerError> {
    let provider = get_provider_http(&env.http_rpc_endpoint);
    let contract_addresses = env.protocol_settings.eigenlayer()?;

    let ecdsa_public = env.keystore().first_local::<K256Ecdsa>()?;
    let ecdsa_secret = env
        .keystore()
        .expose_ecdsa_secret(&ecdsa_public)?
        .ok_or_else(|| RunnerError::Other("No ECDSA secret found".into()))?;
    let operator_address = ecdsa_secret
        .alloy_address()
        .map_err(|e| RunnerError::Eigenlayer(e.to_string()))?;

    let stake_registry_address = contract_addresses.stake_registry_address;

    let ecdsa_stake_registry = ECDSAStakeRegistry::new(stake_registry_address, provider.clone());

    // Check if the operator has already registered for the service
    match ecdsa_stake_registry
        .operatorRegistered(operator_address)
        .call()
        .await
    {
        Ok(is_registered) => Ok(!is_registered._0),
        Err(e) => Err(RunnerError::Eigenlayer(e.to_string())),
    }
}

#[allow(clippy::too_many_lines)]
async fn register_ecdsa_impl(
    env: &BlueprintEnvironment,
    earnings_receiver_address: Address,
    delegation_approver_address: Address,
) -> Result<(), RunnerError> {
    let contract_addresses = env.protocol_settings.eigenlayer()?;
    let registry_coordinator_address = contract_addresses.registry_coordinator_address;
    let allocation_manager_address = contract_addresses.allocation_manager_address;
    let delegation_manager_address = contract_addresses.delegation_manager_address;
    let strategy_manager_address = contract_addresses.strategy_manager_address;
    let avs_directory_address = contract_addresses.avs_directory_address;
    let service_manager_address = contract_addresses.service_manager_address;
    let stake_registry_address = contract_addresses.stake_registry_address;
    let rewards_coordinator_address = contract_addresses.rewards_coordinator_address;
    let permission_controller_address = contract_addresses.permission_controller_address;

    let ecdsa_public = env.keystore().first_local::<K256Ecdsa>()?;
    let ecdsa_secret = env
        .keystore()
        .expose_ecdsa_secret(&ecdsa_public)?
        .ok_or_else(|| RunnerError::Other("No ECDSA secret found".into()))?;
    let operator_address = ecdsa_secret
        .alloy_address()
        .map_err(|e| RunnerError::Eigenlayer(e.to_string()))?;

    let operator_private_key = hex::encode(ecdsa_secret.0.to_bytes());
    blueprint_core::info!("Operator private key: {}", operator_private_key);
    let wallet = PrivateKeySigner::from_str(&operator_private_key)
        .map_err(|_| RunnerError::Other("Invalid private key".into()))?;

    let provider = get_provider_http(&env.http_rpc_endpoint);

    let logger = get_test_logger();
    let el_chain_reader = ELChainReader::new(
        logger,
        Some(allocation_manager_address),
        delegation_manager_address,
        rewards_coordinator_address,
        avs_directory_address,
        Some(permission_controller_address),
        env.http_rpc_endpoint.clone(),
    );

    let el_writer = ELChainWriter::new(
        strategy_manager_address,
        rewards_coordinator_address,
        Some(permission_controller_address),
        Some(allocation_manager_address),
        registry_coordinator_address,
        el_chain_reader.clone(),
        env.http_rpc_endpoint.clone(),
        operator_private_key.clone(),
    );

    let staker_opt_out_window_blocks = 50400u32;
    let operator_details = Operator {
        address: operator_address,
        delegation_approver_address,
        metadata_url: "https://github.com/tangle-network/gadget".to_string(),
        allocation_delay: Some(30), // TODO: Make allocation delay configurable
        _deprecated_earnings_receiver_address: Some(earnings_receiver_address),
        staker_opt_out_window_blocks: Some(staker_opt_out_window_blocks),
    };

    let tx_hash = el_writer
        .register_as_operator(operator_details)
        .await
        .map_err(|e| RunnerError::Eigenlayer(e.to_string()))?;

    blueprint_core::info!("Registered as operator for Eigenlayer {:?}", tx_hash);

    let digest_hash_salt: FixedBytes<32> = FixedBytes::from([0x02; 32]);
    let now = std::time::SystemTime::now();
    let sig_expiry = now.duration_since(std::time::UNIX_EPOCH).map_or_else(
        |_| U256::from(0),
        |duration| U256::from(duration.as_secs()) + U256::from(3600),
    );

    blueprint_core::info!(
        "Registration parameters: operator={:?}, manager={:?}, salt={:?}, expiry={:?}",
        operator_address,
        service_manager_address,
        digest_hash_salt,
        sig_expiry
    );

    let msg_to_sign = el_chain_reader
        .calculate_operator_avs_registration_digest_hash(
            operator_address,
            service_manager_address,
            digest_hash_salt,
            sig_expiry,
        )
        .await
        .map_err(|e| RunnerError::Other(e.to_string()))?;

    let operator_signature = wallet
        .sign_hash(&msg_to_sign)
        .await
        .map_err(|e| RunnerError::SignatureError(e.to_string()))?;
    let signing_key_address = wallet.address();

    let operator_signature_with_salt_and_expiry = SignatureWithSaltAndExpiry {
        signature: operator_signature.as_bytes().into(),
        salt: digest_hash_salt,
        expiry: sig_expiry,
    };

    // --- Register the operator to AVS ---

    let ecdsa_stake_registry = ECDSAStakeRegistry::new(stake_registry_address, provider.clone());

    let _register_response = ecdsa_stake_registry
        .registerOperatorWithSignature(
            operator_signature_with_salt_and_expiry.clone(),
            signing_key_address,
        )
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    let is_registered = ecdsa_stake_registry
        .operatorRegistered(operator_address)
        .call()
        .await
        .map_err(|e| RunnerError::Eigenlayer(format!("Failed to check registration: {}", e)))?;

    blueprint_core::info!("Operator Registration Status {:?}", is_registered._0);
    Ok(())
}
