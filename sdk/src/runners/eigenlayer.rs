use alloy_primitives::{Bytes, FixedBytes, U256};
use eigensdk::{
    client_avsregistry::writer::AvsRegistryChainWriter,
    client_elcontracts::{reader::ELChainReader, writer::ELChainWriter},
    logging::get_logger,
    types::operator::Operator,
};

use crate::{
    config::GadgetConfiguration, events_watcher::evm::get_provider_http, info, keystore::BackendExt,
};

use super::{BlueprintConfig, RunnerError};

#[derive(Clone, Copy)]
pub struct EigenlayerConfig {}

#[async_trait::async_trait]
impl BlueprintConfig for EigenlayerConfig {
    async fn requires_registration(
        &self,
        env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<bool, RunnerError> {
        // Check if the operator has already registered for the service
        let eigenlayer_contracts = env.eigenlayer_contract_addrs;
        let operator = env.keystore()?.ecdsa_key()?;
        let operator_address = operator.alloy_key()?.address();

        let avs_registry_reader = eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
            get_logger(),
            eigenlayer_contracts.registry_coordinator_addr,
            eigenlayer_contracts.operator_state_retriever_addr,
            env.http_rpc_endpoint.clone(),
        ).await?;

        match avs_registry_reader.is_operator_registered(operator_address).await {
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

        let eigenlayer_contracts = env.eigenlayer_contract_addrs;
        let operator = env.keystore()?.ecdsa_key()?;
        let operator_private_key = hex::encode(operator.signer().seed());
        let operator_address = operator.alloy_key()?.address();
        let provider = get_provider_http(&env.http_rpc_endpoint);

        let delegation_manager = eigensdk::utils::binding::DelegationManager::new(
            eigenlayer_contracts.delegation_manager_addr,
            provider.clone(),
        );
        let slasher_address = delegation_manager.slasher().call().await.map(|a| a._0)?;

        let logger = get_logger();
        let avs_registry_writer = AvsRegistryChainWriter::build_avs_registry_chain_writer(
            logger.clone(),
            env.http_rpc_endpoint.clone(),
            operator_private_key.clone(),
            eigenlayer_contracts.registry_coordinator_addr,
            eigenlayer_contracts.operator_state_retriever_addr,
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
            eigenlayer_contracts.delegation_manager_addr,
            eigenlayer_contracts.avs_directory_addr,
            env.http_rpc_endpoint.clone(),
        );

        let el_writer = ELChainWriter::new(
            eigenlayer_contracts.delegation_manager_addr,
            eigenlayer_contracts.strategy_manager_addr,
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
