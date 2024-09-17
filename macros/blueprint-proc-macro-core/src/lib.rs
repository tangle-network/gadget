pub type BlueprintString<'a> = std::borrow::Cow<'a, str>;
/// A type that represents an EVM Address.
pub type Address = ethereum_types::H160;

#[derive(Default, Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub enum FieldType {
    /// A Field of `void` type.
    #[default]
    Void,
    /// A Field of `bool` type.
    Bool,
    /// A Field of `u8` type.
    Uint8,
    /// A Field of `i8` type.
    Int8,
    /// A Field of `u16` type.
    Uint16,
    /// A Field of `i16` type.
    Int16,
    /// A Field of `u32` type.
    Uint32,
    /// A Field of `i32` type.
    Int32,
    /// A Field of `u64` type.
    Uint64,
    /// A Field of `i64` type.
    Int64,
    /// A field of `u128` type.
    Uint128,
    /// A field of `i128` type.
    Int128,
    /// A field of `f64` type.
    Float64,
    /// A Field of `String` type.
    String,
    /// A Field of `Vec<u8>` type.
    Bytes,
    /// A Field of `Option<T>` type.
    Optional(Box<FieldType>),
    /// An array of N items of type [`FieldType`].
    Array(u64, Box<FieldType>),
    /// A List of items of type [`FieldType`].
    List(Box<FieldType>),
    /// A Struct of items of type [`FieldType`].
    Struct(String, Vec<(String, Box<FieldType>)>),
    // NOTE: Special types starts from 100
    /// A special type for AccountId
    AccountId,
}

/// The main definition of a service.
///
/// This contains the metadata of the service, the job definitions, and other hooks, along with the
/// gadget that will be executed when one of the jobs is calling this service.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ServiceBlueprint<'a> {
    /// The metadata of the service.
    pub metadata: ServiceMetadata<'a>,
    /// The job definitions that are available in this service.
    pub jobs: Vec<JobDefinition<'a>>,
    /// The registration hook that will be called before restaker registration.
    pub registration_hook: ServiceRegistrationHook,
    /// The parameters that are required for the service registration.
    pub registration_params: Vec<FieldType>,
    /// The request hook that will be called before creating a service from the service blueprint.
    pub request_hook: ServiceRequestHook,
    /// The parameters that are required for the service request.
    pub request_params: Vec<FieldType>,
    /// The gadget that will be executed for the service.
    pub gadget: Gadget<'a>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ServiceMetadata<'a> {
    /// The Service name.
    pub name: BlueprintString<'a>,
    /// The Service description.
    pub description: Option<BlueprintString<'a>>,
    /// The Service author.
    /// Could be a company or a person.
    pub author: Option<BlueprintString<'a>>,
    /// The Job category.
    pub category: Option<BlueprintString<'a>>,
    /// Code Repository URL.
    /// Could be a github, gitlab, or any other code repository.
    pub code_repository: Option<BlueprintString<'a>>,
    /// Service Logo URL.
    pub logo: Option<BlueprintString<'a>>,
    /// Service Website URL.
    pub website: Option<BlueprintString<'a>>,
    /// Service License.
    pub license: Option<BlueprintString<'a>>,
}

/// A Job Definition is a definition of a job that can be called.
/// It contains the input and output fields of the job with the permitted caller.
#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JobDefinition<'a> {
    /// The metadata of the job.
    pub metadata: JobMetadata<'a>,
    /// These are parameters that are required for this job.
    /// i.e. the input.
    pub params: Vec<FieldType>,
    /// These are the result, the return values of this job.
    /// i.e. the output.
    pub result: Vec<FieldType>,
    /// The verifier of the job result.
    pub verifier: JobResultVerifier,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JobMetadata<'a> {
    /// The Job name.
    pub name: BlueprintString<'a>,
    /// The Job description.
    pub description: Option<BlueprintString<'a>>,
}

/// A Job Result verifier is a verifier that will verify the result of a job call
/// using different verification methods.
#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JobResultVerifier {
    /// No verification is needed.
    #[default]
    None,
    /// An EVM Contract Address or path to the contract ABI that will verify the result.
    Evm(String),
}

/// Represents the definition of a report, including its metadata, parameters, and result type.
#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReportDefinition<'a> {
    /// Metadata about the report, including its name and description.
    pub metadata: ReportMetadata<'a>,

    /// List of parameter types for the report function.
    pub params: Vec<FieldType>,

    /// List of result types for the report function.
    pub result: Vec<FieldType>,

    /// The type of report (Job or QoS).
    pub report_type: ReportType,

    /// The ID of the job this report is associated with (for job reports only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<u8>,

    /// The interval at which this report should be run (for QoS reports only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u64>,

    /// Optional metric thresholds for QoS reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_thresholds: Option<Vec<(String, u64)>>,

    /// The verifier to use for this report's results.
    pub verifier: ReportResultVerifier,
}

/// Enum representing the type of report (Job or QoS).
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
    pub name: BlueprintString<'a>,
    /// The Job description.
    pub description: Option<BlueprintString<'a>>,
}

/// Service Registration hook is a hook that will be called before registering the restaker as
/// an operator for the service.
#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ServiceRegistrationHook {
    /// No hook is needed, the restaker will be registered immediately.
    #[default]
    None,
    /// A Smart contract that will be called to determine if the restaker will be registered.
    Evm(String),
}

/// Service Request hook is a hook that will be called before creating a service from the service blueprint.
#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ServiceRequestHook {
    /// No hook is needed, the caller will get the service created immediately.
    #[default]
    None,
    /// A Smart contract that will be called to determine if the caller meets the requirements to create a service.
    Evm(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Gadget<'a> {
    /// A Gadget that is a WASM binary that will be executed.
    /// inside the shell using the wasm runtime.
    Wasm(WasmGadget<'a>),
    /// A Gadget that is a native binary that will be executed.
    /// inside the shell using the OS.
    Native(NativeGadget<'a>),
    /// A Gadget that is a container that will be executed.
    /// inside the shell using the container runtime (e.g. Docker, Podman, etc.)
    Container(ContainerGadget<'a>),
}

/// A binary that is stored in the GitHub release.
///
/// This will construct the URL to the release and download the binary.
/// The URL will be in the following format:
///
/// `https://github.com/<owner>/<repo>/releases/download/v<tag>/<path>`
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GithubFetcher<'a> {
    /// The owner of the repository.
    pub owner: BlueprintString<'a>,
    /// The repository name.
    pub repo: BlueprintString<'a>,
    /// The release tag of the repository.
    /// NOTE: The tag should be a valid semver tag.
    pub tag: BlueprintString<'a>,
    /// The names of the binary in the release by the arch and the os.
    pub binaries: Vec<GadgetBinary<'a>>,
}

/// The CPU or System architecture.
#[derive(
    PartialEq, PartialOrd, Ord, Eq, Debug, Clone, Copy, serde::Serialize, serde::Deserialize,
)]
pub enum Architecture {
    /// WebAssembly architecture (32-bit).
    Wasm,
    /// WebAssembly architecture (64-bit).
    Wasm64,
    /// WASI architecture (32-bit).
    Wasi,
    /// WASI architecture (64-bit).
    Wasi64,
    /// Amd architecture (32-bit).
    Amd,
    /// Amd64 architecture (x86_64).
    Amd64,
    /// Arm architecture (32-bit).
    Arm,
    /// Arm64 architecture (64-bit).
    Arm64,
    /// Risc-V architecture (32-bit).
    RiscV,
    /// Risc-V architecture (64-bit).
    RiscV64,
}

/// Operating System that the binary is compiled for.
#[derive(
    Default,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Debug,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum OperatingSystem {
    /// Unknown operating system.
    /// This is used when the operating system is not known
    /// for example, for WASM, where the OS is not relevant.
    #[default]
    Unknown,
    /// Linux operating system.
    Linux,
    /// Windows operating system.
    Windows,
    /// MacOS operating system.
    MacOS,
    /// BSD operating system.
    BSD,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GadgetBinary<'a> {
    /// CPU or System architecture.
    pub arch: Architecture,
    /// Operating System that the binary is compiled for.
    pub os: OperatingSystem,
    /// The name of the binary.
    pub name: BlueprintString<'a>,
    /// The sha256 hash of the binary.
    /// used to verify the downloaded binary.
    #[serde(default)]
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(from = "GadgetSourceFlat<'_>")]
pub struct GadgetSource<'a> {
    /// The fetcher that will fetch the gadget from a remote source.
    pub fetcher: GadgetSourceFetcher<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
struct GadgetSourceFlat<'a>(GadgetSourceFetcher<'a>);

impl<'a> From<GadgetSourceFlat<'a>> for GadgetSource<'a> {
    fn from(flat: GadgetSourceFlat<'a>) -> GadgetSource<'a> {
        Self { fetcher: flat.0 }
    }
}

/// A Gadget Source Fetcher is a fetcher that will fetch the gadget
/// from a remote source.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(from = "UntaggedGadgetSourceFetcher<'_>")]
pub enum GadgetSourceFetcher<'a> {
    /// A Gadget that will be fetched from the IPFS.
    IPFS(Vec<u8>),
    /// A Gadget that will be fetched from the Github release.
    Github(GithubFetcher<'a>),
    /// A Gadgets that will be fetched from the container registry.
    ContainerImage(ImageRegistryFetcher<'a>),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct CidWrapper(cid::Cid);

impl<'de> serde::Deserialize<'de> for CidWrapper {
    fn deserialize<D>(deserializer: D) -> Result<CidWrapper, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let str_value = String::deserialize(deserializer)?;
        let cid = cid::Cid::try_from(str_value).map_err(serde::de::Error::custom)?;
        Ok(CidWrapper(cid))
    }
}

/// A Gadget Source Fetcher is a fetcher that will fetch the gadget
/// from a remote source.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum UntaggedGadgetSourceFetcher<'a> {
    /// A Gadget that will be fetched from the IPFS.
    IPFS(CidWrapper),
    /// A Gadget that will be fetched from the Github release.
    Github(GithubFetcher<'a>),
    /// A Gadgets that will be fetched from the container registry.
    ContainerImage(ImageRegistryFetcher<'a>),
}

impl<'a> From<UntaggedGadgetSourceFetcher<'a>> for GadgetSourceFetcher<'a> {
    fn from(tagged: UntaggedGadgetSourceFetcher<'a>) -> GadgetSourceFetcher<'a> {
        match tagged {
            UntaggedGadgetSourceFetcher::IPFS(hash) => GadgetSourceFetcher::IPFS(hash.0.to_bytes()),
            UntaggedGadgetSourceFetcher::Github(fetcher) => GadgetSourceFetcher::Github(fetcher),
            UntaggedGadgetSourceFetcher::ContainerImage(fetcher) => {
                GadgetSourceFetcher::ContainerImage(fetcher)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImageRegistryFetcher<'a> {
    /// The URL of the container registry.
    registry: BlueprintString<'a>,
    /// The name of the image.
    image: BlueprintString<'a>,
    /// The tag of the image.
    tag: BlueprintString<'a>,
}

/// A WASM binary that contains all the compiled gadget code.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WasmGadget<'a> {
    /// Which runtime to use to execute the WASM binary.
    pub runtime: WasmRuntime,
    /// Where the WASM binary is stored.
    pub sources: Vec<GadgetSource<'a>>,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WasmRuntime {
    /// The WASM binary will be executed using the WASMtime runtime.
    Wasmtime,
    /// The WASM binary will be executed using the Wasmer runtime.
    Wasmer,
}

/// A Native binary that contains all the gadget code.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NativeGadget<'a> {
    /// Where the WASM binary is stored.
    pub sources: Vec<GadgetSource<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContainerGadget<'a> {
    /// Where the Image of the gadget binary is stored.
    pub sources: Vec<GadgetSource<'a>>,
}
