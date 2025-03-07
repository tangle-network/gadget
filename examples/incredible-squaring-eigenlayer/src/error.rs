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
