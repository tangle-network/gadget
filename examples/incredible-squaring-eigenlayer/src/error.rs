use blueprint_sdk::IntoJobResult;
use eigensdk::services_blsaggregation::bls_aggregation_service_error::BlsAggregationServiceError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum TaskError {
    #[error(transparent)]
    SolType(#[from] alloy_sol_types::Error),
    #[error(transparent)]
    BlsAggregationService(#[from] BlsAggregationServiceError),
    #[error("Aggregated response receiver closed")]
    AggregatedResponseReceiverClosed,
}

impl IntoJobResult for TaskError {
    fn into_job_result(self) -> Option<blueprint_sdk::JobResult> {
        // Here, I will log the error and return None to indicate that the job has failed and the error has been logged
        tracing::error!("Task failed: {}", self);
        None
    }
}
