use alloc::borrow::Cow;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::FieldType;

/// A Job Definition is a definition of a job that can be called.
/// It contains the input and output fields of the job with the permitted caller.
#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JobDefinition<'a> {
    pub job_id: u64,
    /// The metadata of the job.
    pub metadata: JobMetadata<'a>,
    /// These are parameters that are required for this job.
    /// i.e. the input.
    pub params: Vec<FieldType>,
    /// These are the result, the return values of this job.
    /// i.e. the output.
    pub result: Vec<FieldType>,
}

impl From<tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::jobs::JobDefinition> for JobDefinition<'static> {
    fn from(value: tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::jobs::JobDefinition) -> Self {
        Self {
            job_id: 0,
            metadata: value.metadata.into(),
            params: value.params.0,
            result: value.result.0,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JobMetadata<'a> {
    /// The Job name.
    pub name: Cow<'a, str>,
    /// The Job description.
    pub description: Option<Cow<'a, str>>,
}

impl From<tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::jobs::JobMetadata> for JobMetadata<'static> {
    fn from(value: tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::jobs::JobMetadata) -> Self {
        Self {
            name: String::from_utf8_lossy(&value.name.0.0).into_owned().into(),
            description: value.description.map(|desc| String::from_utf8_lossy(&desc.0.0).into_owned().into()),
        }
    }
}
