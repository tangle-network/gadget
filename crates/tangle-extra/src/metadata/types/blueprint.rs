use alloc::borrow::Cow;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::FieldType;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::service::MasterBlueprintServiceManagerRevision;
use super::job::JobDefinition;
use super::gadget::Gadget;

/// Mirror of [`ServiceBlueprint`] for un-deployed blueprints
///
/// This only exists, as the [`ServiceBlueprint`] uses `Vec<u8>` instead of `String` for string fields,
/// and expects an address for the manager contract, but we haven't yet deployed.
///
/// This needs to be kept up to date to reflect [`ServiceBlueprint`] otherwise.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ServiceBlueprint<'a> {
    /// The metadata of the service.
    pub metadata: ServiceMetadata<'a>,
    /// The blueprint manager that will be used to manage the blueprints lifecycle.
    pub manager: BlueprintServiceManager,
    /// The Revision number of the Master Blueprint Service Manager.
    ///
    /// If not sure what to use, use `MasterBlueprintServiceManagerRevision::default()` which will use
    /// the latest revision available.
    pub master_manager_revision: MasterBlueprintServiceManagerRevision,
    /// The job definitions that are available in this service.
    pub jobs: Vec<JobDefinition<'a>>,
    /// The parameters that are required for the service registration.
    pub registration_params: Vec<FieldType>,
    /// The parameters that are required for the service request.
    pub request_params: Vec<FieldType>,
    /// The gadget that will be executed for the service.
    pub gadget: Gadget<'a>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ServiceMetadata<'a> {
    /// The Service name.
    pub name: Cow<'a, str>,
    /// The Service description.
    pub description: Option<Cow<'a, str>>,
    /// The Service author.
    /// Could be a company or a person.
    pub author: Option<Cow<'a, str>>,
    /// The Job category.
    pub category: Option<Cow<'a, str>>,
    /// Code Repository URL.
    /// Could be a github, gitlab, or any other code repository.
    pub code_repository: Option<Cow<'a, str>>,
    /// Service Logo URL.
    pub logo: Option<Cow<'a, str>>,
    /// Service Website URL.
    pub website: Option<Cow<'a, str>>,
    /// Service License.
    pub license: Option<Cow<'a, str>>,
}

/// Service Blueprint Manager is a smart contract that will manage the service lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum BlueprintServiceManager {
    /// A Smart contract that will manage the service lifecycle.
    Evm(String),
}
