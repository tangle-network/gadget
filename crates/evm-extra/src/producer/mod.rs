//! Event producers for EVM
//!
//! Provides both polling and subscription-based producers for EVM events.

mod polling;

use std::collections::BTreeMap;

use alloy_primitives::FixedBytes;
use alloy_rpc_types::Log;
use blueprint_core::{JobCall, extensions::Extensions, job_call::Parts, metadata::MetadataMap};
pub use polling::{PollingConfig, PollingProducer};

/// Converts Logs to JobCalls
pub(crate) fn logs_to_job_calls(logs: Vec<Log>) -> Vec<JobCall> {
    let mut job_calls = Vec::new();
    let mut logs_by_block = BTreeMap::new();
    for log in logs {
        if let Some(block_number) = log.block_number {
            logs_by_block
                .entry(block_number)
                .or_insert(Vec::new())
                .push(log);
        } else {
            tracing::warn!(?log, "Missing block number");
            continue;
        }
    }

    for (block_number, logs) in logs_by_block {
        let mut extensions = Extensions::new();
        let metadata = MetadataMap::new();
        // TODO: add block number and block hash to the metadata
        extensions.insert(logs.clone());
        tracing::trace!(?block_number, "Processing logs");

        for log in logs {
            let id = match log.topic0() {
                Some(id) => *id,
                // anonymous event
                None => FixedBytes::<32>::default(),
            };
            let parts = Parts::new(*id)
                .with_metadata(metadata.clone())
                .with_extensions(extensions.clone());
            job_calls.push(JobCall::from_parts(parts, Default::default()));
        }
    }
    job_calls
}
