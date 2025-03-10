//! Extractor for EVM transactions.
//!
//! This module contains the extractor for EVM transactions. It is responsible for converting a transaction request into a job result that can be sent to the network.
use alloy_consensus::Transaction;
use alloy_rpc_types::TransactionRequest;
use blueprint_core::IntoJobResult;

/// A Wrapper around a transaction request that will be signed and sent to the network.
#[derive(Debug, Clone)]
pub struct Tx(pub TransactionRequest);

impl IntoJobResult for Tx {
    fn into_job_result(self) -> Option<blueprint_core::JobResult> {
        let tx_bytes = serde_json::to_vec(&self.0)
            .inspect_err(|e| {
                blueprint_core::error!("Failed to serialize transaction: {e}, details: {e:?}");
            })
            .ok()?;
        let job_result = blueprint_core::JobResult::new(tx_bytes.into());
        Some(job_result)
    }
}

impl<T: Transaction> From<T> for Tx {
    fn from(tx: T) -> Self {
        Tx(TransactionRequest::from_transaction(tx))
    }
}
