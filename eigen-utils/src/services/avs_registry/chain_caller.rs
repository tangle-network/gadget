use alloy_network::Ethereum;
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::Provider;
use alloy_transport::Transport;
use async_trait::async_trait;
use eigen_contracts::OperatorStateRetriever;
use std::collections::HashMap;

use crate::avs_registry::reader::AvsRegistryChainReader;
use crate::crypto::bls::G1Point;
use crate::services::operator_info::OperatorInfoServiceTrait;
use crate::types::{
    bytes_to_quorum_ids, OperatorAvsState, OperatorId, OperatorInfo, QuorumAvsState, QuorumNum,
    QuorumNums,
};

use super::AvsRegistryServiceTrait;

#[derive(Debug, Clone)]
pub struct AvsRegistryServiceChainCaller<T, P, I>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
    I: OperatorInfoServiceTrait,
{
    avs_registry_reader: AvsRegistryChainReader<T, P>,
    operator_info_service: I,
}

impl<T, P, I> AvsRegistryServiceChainCaller<T, P, I>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
    I: OperatorInfoServiceTrait,
{
    pub fn new(
        operator_info_service: I,
        avs_registry_reader: AvsRegistryChainReader<T, P>,
    ) -> Self {
        AvsRegistryServiceChainCaller {
            operator_info_service,
            avs_registry_reader,
        }
    }

    pub async fn build(
        registry_coordinator_addr: Address,
        operator_state_retriever_addr: Address,
        eth_client: P,
        operator_info_service: I,
    ) -> Self {
        let avs_registry_reader = AvsRegistryChainReader::build(
            registry_coordinator_addr,
            operator_state_retriever_addr,
            eth_client,
        )
        .await;
        AvsRegistryServiceChainCaller::new(operator_info_service, avs_registry_reader)
    }

    async fn get_operator_info(&self, operator_id: B256) -> Result<OperatorInfo, String> {
        let operator_addr = self
            .avs_registry_reader
            .get_operator_from_id(operator_id)
            .await
            .map_err(|e| format!("Failed to get operator address from pubkey hash: {:?}", e))?;

        self.operator_info_service
            .get_operator_info(operator_addr)
            .await
            .ok_or_else(|| format!("Failed to get operator info from operatorInfoService (operatorAddr: {:?}, operatorId: {:?})", operator_addr, operator_id))
    }
}

#[async_trait]
impl<T, P, I> AvsRegistryServiceTrait for AvsRegistryServiceChainCaller<T, P, I>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone + 'static,
    I: OperatorInfoServiceTrait,
{
    async fn get_operators_avs_state_at_block(
        &self,
        quorum_numbers: Bytes,
        block_number: u64,
    ) -> Result<HashMap<OperatorId, OperatorAvsState>, String> {
        let mut operators_avs_state: HashMap<OperatorId, OperatorAvsState> = HashMap::new();

        let operators_stakes_in_quorums = self
            .avs_registry_reader
            .get_operators_stake_in_quorums_at_block(quorum_numbers.clone(), block_number)
            .await
            .map_err(|e| format!("Failed to get operator state: {:?}", e))?;

        let quorum_nums_vec: Vec<QuorumNum> = bytes_to_quorum_ids(&quorum_numbers);
        if operators_stakes_in_quorums.len() != quorum_nums_vec.clone().len() {
            log::error!("Number of quorums returned from GetOperatorsStakeInQuorumsAtBlock does not match number of quorums requested. Probably pointing to old contract or wrong implementation.");
        }

        for (quorum_idx, quorum_num) in quorum_nums_vec.iter().enumerate() {
            for operator in &operators_stakes_in_quorums[quorum_idx] {
                let info = self.get_operator_info(operator.operatorId).await?;
                let operator_stake = U256::from(operator.stake);
                if let Some(operator_avs_state) = operators_avs_state.get_mut(&operator.operatorId)
                {
                    operator_avs_state
                        .stake_per_quorum
                        .insert(quorum_num.clone(), operator_stake);
                } else {
                    let mut stake_per_quorum = HashMap::new();
                    stake_per_quorum.insert(quorum_num.clone(), operator_stake);
                    operators_avs_state.insert(
                        operator.operatorId,
                        OperatorAvsState {
                            operator_id: operator.operatorId,
                            operator_info: info,
                            stake_per_quorum,
                            block_number: block_number.try_into().unwrap(),
                        },
                    );
                }
            }
        }

        Ok(operators_avs_state)
    }

    async fn get_quorums_avs_state_at_block(
        &self,
        quorum_numbers: Bytes,
        block_number: u64,
    ) -> Result<HashMap<QuorumNum, QuorumAvsState>, String> {
        let operators_avs_state = self
            .get_operators_avs_state_at_block(quorum_numbers.clone(), block_number)
            .await?;

        let mut quorums_avs_state = HashMap::new();

        let quorum_num_vec: QuorumNums = bytes_to_quorum_ids(&quorum_numbers);
        for quorum_num in quorum_num_vec {
            let mut agg_pubkey_g1 = G1Point::zero();
            let mut total_stake = U256::from(0);

            for operator in operators_avs_state.values() {
                if let Some(stake) = operator.stake_per_quorum.get(&quorum_num) {
                    agg_pubkey_g1.add(&G1Point::from_ark_g1(
                        &operator.operator_info.pubkeys.g1_pubkey,
                    ));
                    total_stake += stake;
                }
            }

            quorums_avs_state.insert(
                quorum_num.clone(),
                QuorumAvsState {
                    quorum_number: quorum_num,
                    agg_pubkey_g1,
                    total_stake,
                    block_number: block_number.try_into().unwrap(),
                },
            );
        }

        Ok(quorums_avs_state)
    }

    async fn get_check_signatures_indices(
        &self,
        reference_block_number: u64,
        quorum_numbers: Bytes,
        non_signer_operator_ids: Vec<OperatorId>,
    ) -> Result<OperatorStateRetriever::CheckSignaturesIndices, String> {
        self.avs_registry_reader
            .get_check_signatures_indices(
                reference_block_number.try_into().unwrap(),
                quorum_numbers,
                non_signer_operator_ids,
            )
            .await
            .map_err(|e| format!("Failed to get check signatures indices: {:?}", e))
    }
}
