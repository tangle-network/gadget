use std::str::FromStr;

use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{hex, Address, FixedBytes, U256};
use alloy_provider::Provider;
use alloy_rpc_types::BlockNumberOrTag;
use alloy_signer::Signer;
use alloy_signer_local::PrivateKeySigner;

use eigensdk::client_elcontracts::{reader::ELChainReader, writer::ELChainWriter};
use eigensdk::logging::get_test_logger;
use eigensdk::types::operator::Operator;
use eigensdk::utils::ecdsastakeregistry::{ECDSAStakeRegistry, ISignatureUtils};

use crate::error::EigenlayerError;
use gadget_config::{GadgetConfiguration, ProtocolSettings};
use gadget_runner_core::config::BlueprintConfig;
use gadget_runner_core::error::RunnerError as Error;
use gadget_utils::gadget_utils_evm::get_provider_http;

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
    let contract_addresses = match env.protocol_settings {
        ProtocolSettings::Eigenlayer(addresses) => addresses,
        _ => {
            return Err(gadget_runner_core::error::RunnerError::InvalidProtocol(
                "Expected Eigenlayer protocol".into(),
            ));
        }
    };
    let registry_coordinator_address = contract_addresses.registry_coordinator_address;
    let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;
    let operator = env.keystore()?.ecdsa_key()?;
    let operator_address = operator.alloy_key()?.address();

    let avs_registry_reader = eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
        get_test_logger(),
        registry_coordinator_address,
        operator_state_retriever_address,
        env.http_rpc_endpoint.clone(),
    )
    .await
    .map_err(|e| EigenlayerError::AvsRegistry(e).into())?;

    // Check if the operator has already registered for the service
    match avs_registry_reader
        .is_operator_registered(operator_address)
        .await
    {
        Ok(is_registered) => Ok(!is_registered),
        Err(e) => Err(EigenlayerError::AvsRegistry(e).into()),
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
            return Err(gadget_runner_core::error::RunnerError::InvalidProtocol(
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

    let operator = env
        .keystore()
        .map_err(|e| EigenlayerError::Keystore(e.to_string()))?
        .ecdsa_key()
        .map_err(|e| EigenlayerError::Keystore(e.to_string()))?;

    let operator_private_key = hex::encode(operator.signer().seed());
    let wallet = PrivateKeySigner::from_str(&operator_private_key)
        .map_err(|_| EigenlayerError::Keystore("Invalid private key".into()))?;

    let operator_address = operator
        .address()
        .map_err(|_| EigenlayerError::Keystore("Invalid private key".into()))?;

    let provider = get_provider_http(&env.http_rpc_endpoint);

    let delegation_manager = eigensdk::utils::delegationmanager::DelegationManager::new(
        delegation_manager_address,
        provider.clone(),
    );

    let slasher_address = delegation_manager
        .slasher()
        .call()
        .await
        .map(|a| a._0)
        .map_err(|e| EigenlayerError::Contract(e).into())?;

    let logger = get_test_logger();
    let el_chain_reader = ELChainReader::new(
        logger,
        slasher_address,
        delegation_manager_address,
        avs_directory_address,
        env.http_rpc_endpoint.clone(),
    );

    let el_writer = ELChainWriter::new(
        delegation_manager_address,
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
        .map_err(|e| EigenlayerError::ElContracts(e).into())?;

    gadget_logging::info!("Registered as operator for Eigenlayer {:?}", tx_hash);

    let digest_hash_salt: FixedBytes<32> = FixedBytes::from([0x02; 32]);
    let now = std::time::SystemTime::now();
    let sig_expiry = now
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| U256::from(duration.as_secs()) + U256::from(86400))
        .unwrap_or_else(|_| {
            gadget_logging::info!("System time seems to be before the UNIX epoch.");
            U256::from(0)
        });

    let msg_to_sign = el_chain_reader
        .calculate_operator_avs_registration_digest_hash(
            operator_address,
            service_manager_address,
            digest_hash_salt,
            sig_expiry,
        )
        .await
        .map_err(|e| EigenlayerError::Other(e.to_string()))?;

    let operator_signature = wallet
        .sign_hash(&msg_to_sign)
        .await
        .map_err(|e| Error::SignatureError(e.to_string()))?;

    let operator_signature_with_salt_and_expiry = ISignatureUtils::SignatureWithSaltAndExpiry {
        signature: operator_signature.as_bytes().into(),
        salt: digest_hash_salt,
        expiry: sig_expiry,
    };

    let signer = alloy_signer_local::PrivateKeySigner::from_str(&operator_private_key)
        .map_err(|e| Error::SignatureError(e.to_string()))?;
    let wallet = EthereumWallet::from(signer);

    // --- Register the operator to AVS ---

    gadget_logging::info!("Building Transaction");

    let latest_block_number = provider
        .get_block_number()
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;

    // Get the latest block to estimate gas price
    let latest_block = provider
        .get_block_by_number(BlockNumberOrTag::Number(latest_block_number), false)
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?
        .ok_or(Error::TransactionError("Failed to get latest block".into()))?;

    // Get the base fee per gas from the latest block
    let base_fee_per_gas: u128 = latest_block
        .header
        .base_fee_per_gas
        .ok_or(Error::TransactionError(
            "Failed to get base fee per gas from latest block".into(),
        ))?
        .into();

    // Get the max priority fee per gas
    let max_priority_fee_per_gas = provider
        .get_max_priority_fee_per_gas()
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;

    // Calculate max fee per gas
    let max_fee_per_gas = base_fee_per_gas + max_priority_fee_per_gas;

    // Build the transaction request
    let tx = alloy_rpc_types::TransactionRequest::default()
        .with_call(&ECDSAStakeRegistry::registerOperatorWithSignatureCall {
            _operatorSignature: operator_signature_with_salt_and_expiry,
            _signingKey: operator_address,
        })
        .with_from(operator_address)
        .with_to(stake_registry_address)
        .with_nonce(
            provider
                .get_transaction_count(operator_address)
                .await
                .map_err(|e| Error::TransactionError(e.to_string()))?,
        )
        .with_chain_id(
            provider
                .get_chain_id()
                .await
                .map_err(|e| Error::TransactionError(e.to_string()))?,
        )
        .with_max_priority_fee_per_gas(max_priority_fee_per_gas)
        .with_max_fee_per_gas(max_fee_per_gas);

    // Estimate gas limit
    let gas_estimate = provider
        .estimate_gas(&tx)
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;
    gadget_logging::info!("Gas Estimate: {}", gas_estimate);

    // Set gas limit
    let tx = tx.with_gas_limit(gas_estimate);

    gadget_logging::info!("Building Transaction Envelope");

    let tx_envelope = tx
        .build(&wallet)
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;

    gadget_logging::info!("Sending Transaction Envelope");

    let result = provider
        .send_tx_envelope(tx_envelope)
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?
        .register()
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;

    gadget_logging::info!("Operator Registration to AVS Sent. Awaiting Receipt...");

    gadget_logging::info!("Operator Address: {}", operator_address);
    gadget_logging::info!("Stake Registry Address: {}", stake_registry_address);
    gadget_logging::info!("RPC Endpoint: {}", env.http_rpc_endpoint);

    let tx_hash = result
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?;

    gadget_logging::info!(
        "Command for testing: cast code {} --rpc-url {}",
        stake_registry_address,
        env.http_rpc_endpoint
    );

    let receipt = provider
        .get_transaction_receipt(tx_hash)
        .await
        .map_err(|e| Error::TransactionError(e.to_string()))?
        .ok_or(Error::TransactionError("Failed to get receipt".into()))?;

    gadget_logging::info!("Got Transaction Receipt: {:?}", receipt);

    if !receipt.status() {
        return Err(EigenlayerError::Registration(
            "Failed to register operator to AVS".to_string(),
        )
        .into());
    }

    gadget_logging::info!("Operator Registration to AVS Succeeded");
    Ok(())
}
