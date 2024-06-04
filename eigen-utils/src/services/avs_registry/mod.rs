use alloy_primitives::{Bytes, FixedBytes};
use async_trait::async_trait;
use eigen_contracts::OperatorStateRetriever;
use std::collections::HashMap;

use crate::types::{OperatorAvsState, OperatorId, QuorumAvsState, QuorumNum};

pub mod chain_caller;

#[async_trait]
pub trait AvsRegistryServiceTrait: Send + Sync + Clone + 'static {
    async fn get_operators_avs_state_at_block(
        &self,
        quorum_numbers: Bytes,
        block_number: u64,
    ) -> Result<HashMap<OperatorId, OperatorAvsState>, String>;

    async fn get_quorums_avs_state_at_block(
        &self,
        quorum_numbers: Bytes,
        block_number: u64,
    ) -> Result<HashMap<QuorumNum, QuorumAvsState>, String>;

    async fn get_check_signatures_indices(
        &self,
        reference_block_number: u64,
        quorum_numbers: Bytes,
        non_signer_operator_ids: Vec<FixedBytes<32>>,
    ) -> Result<OperatorStateRetriever::CheckSignaturesIndices, String>;
}
