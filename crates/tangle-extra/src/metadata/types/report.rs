use alloc::borrow::Cow;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::FieldType;

/// Represents the definition of a report, including its metadata, parameters, and result type.
#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReportDefinition<'a> {
    /// Metadata about the report, including its name and description.
    pub metadata: ReportMetadata<'a>,

    /// List of parameter types for the report function.
    pub params: Vec<FieldType>,

    /// List of result types for the report function.
    pub result: Vec<FieldType>,

    /// The type of report (Job or `QoS`).
    pub report_type: ReportType,

    /// The ID of the job this report is associated with (for job reports only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<u8>,

    /// The interval at which this report should be run (for `QoS` reports only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u64>,

    /// Optional metric thresholds for `QoS` reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_thresholds: Option<Vec<(String, u64)>>,

    /// The verifier to use for this report's results.
    pub verifier: ReportResultVerifier,
}

/// Enum representing the type of report (Job or `QoS`).
#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportType {
    /// A report associated with a specific job.
    #[default]
    Job,
    /// A report for Quality of Service metrics.
    QoS,
}

/// Enum representing the type of verifier for the report result.
#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportResultVerifier {
    /// No verifier specified.
    #[default]
    None,
    /// An EVM-based verifier contract.
    Evm(String),
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReportMetadata<'a> {
    /// The Job name.
    pub name: Cow<'a, str>,
    /// The Job description.
    pub description: Option<Cow<'a, str>>,
}
