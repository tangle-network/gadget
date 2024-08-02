pub trait QoSReporter {
    type Metrics;
    type ReportResult;

    async fn collect_metrics(&self) -> Result<Self::Metrics, Box<dyn std::error::Error>>;
    async fn report(
        &self,
        metrics: &Self::Metrics,
    ) -> Result<Self::ReportResult, Box<dyn std::error::Error>>;
}
