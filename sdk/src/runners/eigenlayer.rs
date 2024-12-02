use crate::binding::ecdsa_stake_registry::ECDSAStakeRegistry;
use crate::config::protocol::ProtocolSpecificSettings;
use crate::{
    config::GadgetConfiguration, info, keystore::BackendExt, utils::evm::get_provider_http,
};
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{Bytes, FixedBytes, U256};
use alloy_provider::Provider;
use alloy_rpc_types::BlockNumberOrTag;
use alloy_signer::Signer;
use alloy_signer_local::PrivateKeySigner;
use core::str::FromStr;
use eigensdk::{
    client_avsregistry::writer::AvsRegistryChainWriter,
    client_elcontracts::{reader::ELChainReader, writer::ELChainWriter},
    logging::get_test_logger,
    types::operator::Operator,
};

use super::{BlueprintConfig, RunnerError};

#[derive(Clone, Copy)]
pub struct EigenlayerBLSConfig {}

#[async_trait::async_trait]
impl BlueprintConfig for EigenlayerBLSConfig {
    async fn requires_registration(
        &self,
        env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<bool, RunnerError> {
        if env.skip_registration {
            return Ok(false);
        }

        let ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &env.protocol_specific
        else {
            return Err(RunnerError::InvalidProtocol(
                "Expected Eigenlayer protocol".into(),
            ));
        };
        let registry_coordinator_address = contract_addresses.registry_coordinator_address;
        let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;
        let operator = env.keystore()?.ecdsa_key()?;
        let operator_address = operator.alloy_key()?.address();

        let avs_registry_reader =
            eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
                get_test_logger(),
                registry_coordinator_address,
                operator_state_retriever_address,
                env.http_rpc_endpoint.clone(),
            )
            .await?;

        // Check if the operator has already registered for the service
        match avs_registry_reader
            .is_operator_registered(operator_address)
            .await
        {
            Ok(is_registered) => Ok(!is_registered),
            Err(e) => Err(RunnerError::AvsRegistryError(e)),
        }
    }

    async fn register(
        &self,
        env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<(), RunnerError> {
        if env.test_mode {
            info!("Skipping registration in test mode");
            return Ok(());
        }

        let ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &env.protocol_specific
        else {
            return Err(RunnerError::InvalidProtocol(
                "Expected Eigenlayer protocol".into(),
            ));
        };

        let registry_coordinator_address = contract_addresses.registry_coordinator_address;
        let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;
        let delegation_manager_address = contract_addresses.delegation_manager_address;
        let strategy_manager_address = contract_addresses.strategy_manager_address;
        let rewards_coordinator_address = contract_addresses.rewards_coordinator_address;
        let avs_directory_address = contract_addresses.avs_directory_address;

        let operator = env.keystore()?.ecdsa_key()?;
        let operator_private_key = hex::encode(operator.signer().seed());
        let operator_address = operator.alloy_key()?.address();
        let provider = get_provider_http(&env.http_rpc_endpoint);

        let delegation_manager =
            eigensdk::utils::delegationmanager::DelegationManager::DelegationManagerInstance::new(
                delegation_manager_address,
                provider.clone(),
            );
        let slasher_address = delegation_manager.slasher().call().await.map(|a| a._0)?;

        let logger = get_test_logger();
        let avs_registry_writer = AvsRegistryChainWriter::build_avs_registry_chain_writer(
            logger.clone(),
            env.http_rpc_endpoint.clone(),
            operator_private_key.clone(),
            registry_coordinator_address,
            operator_state_retriever_address,
        )
        .await
        .expect("avs writer build fail ");

        let operator_bls_key = env.keystore()?.bls_bn254_key()?;
        let digest_hash: FixedBytes<32> = FixedBytes::from([0x02; 32]);

        let now = std::time::SystemTime::now();
        let sig_expiry = now
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| U256::from(duration.as_secs()) + U256::from(86400))
            .unwrap_or_else(|_| {
                info!("System time seems to be before the UNIX epoch.");
                U256::from(0)
            });

        let quorum_nums = Bytes::from(vec![0]);

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
            el_chain_reader,
            env.http_rpc_endpoint.clone(),
            operator_private_key,
        );

        let staker_opt_out_window_blocks = 50400u32;
        let operator_details = Operator {
            address: operator_address,
            earnings_receiver_address: operator_address,
            delegation_approver_address: operator_address,
            metadata_url: Some("https://github.com/tangle-network/gadget".to_string()),
            staker_opt_out_window_blocks,
        };

        let tx_hash = el_writer.register_as_operator(operator_details).await?;
        info!("Registered as operator for Eigenlayer {:?}", tx_hash);

        let tx_hash = avs_registry_writer
            .register_operator_in_quorum_with_avs_registry_coordinator(
                operator_bls_key,
                digest_hash,
                sig_expiry,
                quorum_nums,
                env.http_rpc_endpoint.clone(),
            )
            .await?;

        info!("Registered operator for Eigenlayer {:?}", tx_hash);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct EigenlayerECDSAConfig {}

#[async_trait::async_trait]
impl BlueprintConfig for crate::runners::eigenlayer::EigenlayerECDSAConfig {
    async fn requires_registration(
        &self,
        env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<bool, RunnerError> {
        if env.skip_registration {
            return Ok(false);
        }

        let ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &env.protocol_specific
        else {
            return Err(RunnerError::InvalidProtocol(
                "Expected Eigenlayer protocol".into(),
            ));
        };
        let registry_coordinator_address = contract_addresses.registry_coordinator_address;
        let operator_state_retriever_address = contract_addresses.operator_state_retriever_address;
        let operator = env.keystore()?.ecdsa_key()?;
        let operator_address = operator.alloy_key()?.address();

        let avs_registry_reader =
            eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
                get_test_logger(),
                registry_coordinator_address,
                operator_state_retriever_address,
                env.http_rpc_endpoint.clone(),
            )
            .await?;

        // Check if the operator has already registered for the service
        match avs_registry_reader
            .is_operator_registered(operator_address)
            .await
        {
            Ok(is_registered) => Ok(!is_registered),
            Err(e) => Err(RunnerError::AvsRegistryError(e)),
        }
    }

    async fn register(
        &self,
        env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<(), RunnerError> {
        if env.test_mode {
            info!("Skipping registration in test mode");
            return Ok(());
        }

        let ProtocolSpecificSettings::Eigenlayer(contract_addresses) = &env.protocol_specific
        else {
            return Err(RunnerError::TransactionError(
                "Missing EigenLayer contract addresses".into(),
            ));
        };

        let delegation_manager_address = contract_addresses.delegation_manager_address;
        let strategy_manager_address = contract_addresses.strategy_manager_address;
        let avs_directory_address = contract_addresses.avs_directory_address;
        let service_manager_address = contract_addresses.service_manager_address;
        let stake_registry_address = contract_addresses.stake_registry_address;
        let rewards_coordinator_address = contract_addresses.rewards_coordinator_address;

        let operator = env
            .keystore()
            .map_err(|e| RunnerError::EigenlayerError(e.to_string()))?
            .ecdsa_key()
            .map_err(|e| RunnerError::EigenlayerError(e.to_string()))?;
        let operator_private_key = hex::encode(operator.signer().seed());
        let wallet = PrivateKeySigner::from_str(&operator_private_key)
            .map_err(|_| RunnerError::EigenlayerError("Invalid private key".into()))?;
        let operator_address = operator
            .alloy_key()
            .map_err(|e| RunnerError::EigenlayerError(e.to_string()))?
            .address();
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
            .map_err(|e| RunnerError::EigenlayerError(e.to_string()))?;

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
            earnings_receiver_address: operator_address,
            delegation_approver_address: operator_address,
            metadata_url: Some("https://github.com/tangle-network/gadget".to_string()),
            staker_opt_out_window_blocks,
        };

        let tx_hash = el_writer
            .register_as_operator(operator_details)
            .await
            .map_err(|e| RunnerError::EigenlayerError(e.to_string()))?;
        info!("Registered as operator for Eigenlayer {:?}", tx_hash);

        let digest_hash_salt: FixedBytes<32> = FixedBytes::from([0x02; 32]);
        let now = std::time::SystemTime::now();
        let sig_expiry = now
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| U256::from(duration.as_secs()) + U256::from(86400))
            .unwrap_or_else(|_| {
                info!("System time seems to be before the UNIX epoch.");
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
            .map_err(|_| RunnerError::EigenlayerError("Failed to calculate hash".to_string()))?;

        let operator_signature = wallet
            .sign_hash(&msg_to_sign)
            .await
            .map_err(|e| RunnerError::SignatureError(e.to_string()))?;

        let operator_signature_with_salt_and_expiry =
            ECDSAStakeRegistry::SignatureWithSaltAndExpiry {
                signature: operator_signature.as_bytes().into(),
                salt: digest_hash_salt,
                expiry: sig_expiry,
            };

        let signer = alloy_signer_local::PrivateKeySigner::from_str(&operator_private_key)
            .map_err(|e| RunnerError::SignatureError(e.to_string()))?;
        let wallet = EthereumWallet::from(signer);

        // --- Register the operator to AVS ---

        info!("Building Transaction");

        let latest_block_number = provider
            .get_block_number()
            .await
            .map_err(|e| RunnerError::TransactionError(e.to_string()))?;

        // Get the latest block to estimate gas price
        let latest_block = provider
            .get_block_by_number(BlockNumberOrTag::Number(latest_block_number), false)
            .await
            .map_err(|e| RunnerError::TransactionError(e.to_string()))?
            .ok_or(RunnerError::TransactionError(
                "Failed to get latest block".into(),
            ))?;

        // Get the base fee per gas from the latest block
        let base_fee_per_gas: u128 = latest_block
            .header
            .base_fee_per_gas
            .ok_or(RunnerError::TransactionError(
                "Failed to get base fee per gas from latest block".into(),
            ))?
            .into();

        // Get the max priority fee per gas
        let max_priority_fee_per_gas = provider
            .get_max_priority_fee_per_gas()
            .await
            .map_err(|e| RunnerError::TransactionError(e.to_string()))?;

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
                    .map_err(|e| RunnerError::TransactionError(e.to_string()))?,
            )
            .with_chain_id(
                provider
                    .get_chain_id()
                    .await
                    .map_err(|e| RunnerError::TransactionError(e.to_string()))?,
            )
            .with_max_priority_fee_per_gas(max_priority_fee_per_gas)
            .with_max_fee_per_gas(max_fee_per_gas);

        // Estimate gas limit
        let gas_estimate = provider
            .estimate_gas(&tx)
            .await
            .map_err(|e| RunnerError::TransactionError(e.to_string()))?;
        info!("Gas Estimate: {}", gas_estimate);

        // Set gas limit
        let tx = tx.with_gas_limit(gas_estimate);

        info!("Building Transaction Envelope");

        let tx_envelope = tx
            .build(&wallet)
            .await
            .map_err(|e| RunnerError::TransactionError(e.to_string()))?;

        info!("Sending Transaction Envelope");

        let result = provider
            .send_tx_envelope(tx_envelope)
            .await
            .map_err(|e| RunnerError::TransactionError(e.to_string()))?
            .register()
            .await
            .map_err(|e| RunnerError::TransactionError(e.to_string()))?;

        info!("Operator Registration to AVS Sent. Awaiting Receipt...");

        info!("Operator Address: {}", operator_address);
        info!("Stake Registry Address: {}", stake_registry_address);
        info!("RPC Endpoint: {}", env.http_rpc_endpoint);

        let tx_hash = result
            .await
            .map_err(|e| RunnerError::TransactionError(e.to_string()))?;

        info!(
            "Command for testing: cast code {} --rpc-url {}",
            stake_registry_address, env.http_rpc_endpoint
        );

        let receipt = provider
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(|e| RunnerError::TransactionError(e.to_string()))?
            .ok_or(RunnerError::TransactionError(
                "Failed to get receipt".into(),
            ))?;
        info!("Got Transaction Receipt: {:?}", receipt);

        if !receipt.status() {
            return Err(RunnerError::EigenlayerError(
                "Failed to register operator to AVS".to_string(),
            ));
        }
        info!("Operator Registration to AVS Succeeded");
        Ok(())
    }
}
