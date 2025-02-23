use alloy_primitives::{hex, Address, FixedBytes, U256};
use alloy_signer::Signer;
use alloy_signer_local::PrivateKeySigner;
use eigensdk::client_elcontracts::{reader::ELChainReader, writer::ELChainWriter};
use eigensdk::logging::get_test_logger;
use eigensdk::types::operator::Operator;
use gadget_config::{GadgetConfiguration, ProtocolSettings};
use gadget_contexts::keystore::KeystoreContext;
use gadget_eigenlayer_bindings::ecdsa_stake_registry::ECDSAStakeRegistry;
use gadget_eigenlayer_bindings::ecdsa_stake_registry::ECDSAStakeRegistry::SignatureWithSaltAndExpiry;
use gadget_keystore::backends::eigenlayer::EigenlayerBackend;
use gadget_keystore::backends::Backend;
use gadget_keystore::crypto::k256::K256Ecdsa;
use gadget_logging::info;
use gadget_runner_core::config::BlueprintConfig;
use gadget_runner_core::error::{RunnerError as Error, RunnerError};
use gadget_utils::evm::get_provider_http;
use std::str::FromStr;

#[derive(Clone, Copy)]
pub struct EigenlayerECDSAConfig {
    earnings_receiver_address: Address,
    delegation_approver_address: Address,
}

impl EigenlayerECDSAConfig {
    pub fn new(earnings_receiver_address: Address, delegation_approver_address: Address) -> Self {
        Self {
            earnings_receiver_address,
            delegation_approver_address,
        }
    }
}

#[async_trait::async_trait]
impl BlueprintConfig for EigenlayerECDSAConfig {
    async fn register(&self, env: &GadgetConfiguration) -> Result<(), Error> {
        register_ecdsa_impl(
            env,
            self.earnings_receiver_address,
            self.delegation_approver_address,
        )
        .await
    }

    async fn requires_registration(&self, env: &GadgetConfiguration) -> Result<bool, Error> {
        requires_registration_ecdsa_impl(env).await
    }
}

async fn requires_registration_ecdsa_impl(env: &GadgetConfiguration) -> Result<bool, Error> {
    let provider = get_provider_http(&env.http_rpc_endpoint);
    let contract_addresses = match env.protocol_settings {
        ProtocolSettings::Eigenlayer(addresses) => addresses,
        _ => {
            return Err(Error::InvalidProtocol(
                "Expected Eigenlayer protocol".into(),
            ));
        }
    };
    // let registry_coordinator_address = contract_addresses.registry_coordinator_address;
    // let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;

    let ecdsa_public = env
        .keystore()
        .first_local::<K256Ecdsa>()
        .map_err(|e| Error::Keystore(e.to_string()))?;
    let ecdsa_secret = env
        .keystore()
        .expose_ecdsa_secret(&ecdsa_public)
        .map_err(|e| Error::Keystore(format!("Failed to expose ECDSA secret: {}", e)))?
        .ok_or_else(|| Error::Keystore("No ECDSA secret found".into()))?;
    let operator_address = ecdsa_secret
        .alloy_address()
        .map_err(|e| Error::Eigenlayer(e.to_string()))?;

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

async fn register_ecdsa_impl(
    env: &GadgetConfiguration,
    earnings_receiver_address: Address,
    delegation_approver_address: Address,
) -> Result<(), Error> {
    let contract_addresses = match env.protocol_settings {
        ProtocolSettings::Eigenlayer(addresses) => addresses,
        _ => {
            return Err(RunnerError::InvalidProtocol(
                "Expected Eigenlayer protocol".into(),
            ));
        }
    };
    let delegation_manager_address = contract_addresses.delegation_manager_address;
    let strategy_manager_address = contract_addresses.strategy_manager_address;
    let avs_directory_address = contract_addresses.avs_directory_address;
    let service_manager_address = contract_addresses.service_manager_address;
    let stake_registry_address = contract_addresses.stake_registry_address;
    let rewards_coordinator_address = contract_addresses.rewards_coordinator_address;

    let ecdsa_public = env
        .keystore()
        .first_local::<K256Ecdsa>()
        .map_err(|e| Error::Keystore(e.to_string()))?;
    let ecdsa_secret = env
        .keystore()
        .expose_ecdsa_secret(&ecdsa_public)
        .map_err(|e| Error::Keystore(format!("Failed to expose ECDSA secret: {}", e)))?
        .ok_or_else(|| Error::Keystore("No ECDSA secret found".into()))?;
    let operator_address = ecdsa_secret
        .alloy_address()
        .map_err(|e| Error::Eigenlayer(e.to_string()))?;

    let operator_private_key = hex::encode(ecdsa_secret.0.to_bytes());
    info!("Operator private key: {}", operator_private_key);
    let wallet = PrivateKeySigner::from_str(&operator_private_key)
        .map_err(|_| Error::Keystore("Invalid private key".into()))?;

    let provider = get_provider_http(&env.http_rpc_endpoint);

    let delegation_manager = eigensdk::utils::core::delegationmanager::DelegationManager::new(
        delegation_manager_address,
        provider.clone(),
    );

    let slasher_address = delegation_manager
        .slasher()
        .call()
        .await
        .map(|a| a._0)
        .map_err(|e| RunnerError::Eigenlayer(e.to_string()))?;

    let logger = get_test_logger();
    let el_chain_reader = ELChainReader::new(
        logger,
        slasher_address,
        delegation_manager_address,
        rewards_coordinator_address,
        avs_directory_address,
        env.http_rpc_endpoint.clone(),
    );

    let el_writer = ELChainWriter::new(
        strategy_manager_address,
        rewards_coordinator_address,
        el_chain_reader.clone(),
        env.http_rpc_endpoint.clone(),
        operator_private_key.clone(),
    );

    let staker_opt_out_window_blocks = 50400u32;
    let operator_details = Operator {
        address: operator_address,
        earnings_receiver_address,
        delegation_approver_address,
        metadata_url: Some("https://github.com/tangle-network/gadget".to_string()),
        staker_opt_out_window_blocks,
    };

    let tx_hash = el_writer
        .register_as_operator(operator_details)
        .await
        .map_err(|e| RunnerError::Eigenlayer(e.to_string()))?;

    info!("Registered as operator for Eigenlayer {:?}", tx_hash);

    let digest_hash_salt: FixedBytes<32> = FixedBytes::from([0x02; 32]);
    let now = std::time::SystemTime::now();
    let sig_expiry = now
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| U256::from(duration.as_secs()) + U256::from(3600))
        .unwrap_or_else(|_| U256::from(0));

    info!(
        "Registration parameters: operator={:?}, manager={:?}, salt={:?}, expiry={:?}",
        operator_address, service_manager_address, digest_hash_salt, sig_expiry
    );

    let msg_to_sign = el_chain_reader
        .calculate_operator_avs_registration_digest_hash(
            operator_address,
            service_manager_address,
            digest_hash_salt,
            sig_expiry,
        )
        .await
        .map_err(|e| Error::Other(e.to_string()))?;

    let operator_signature = wallet
        .sign_hash(&msg_to_sign)
        .await
        .map_err(|e| Error::SignatureError(e.to_string()))?;
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
        .map_err(|e| Error::Eigenlayer(format!("Failed to check registration: {}", e)))?;

    info!("Operator Registration Status {:?}", is_registered._0);
    Ok(())
}
