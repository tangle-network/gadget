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
    /// A field of `u256` type
    U256,
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
    /// Tuple
    Tuple(Vec<FieldType>),
    // NOTE: Special types starts from 100
    /// A special type for AccountId
    AccountId,
}

impl AsRef<str> for FieldType {
    fn as_ref(&self) -> &str {
        match self {
            FieldType::Uint8 => "u8",
            FieldType::Uint16 => "u16",
            FieldType::Uint32 => "u32",
            FieldType::Uint64 => "u64",
            FieldType::Int8 => "i8",
            FieldType::Int16 => "i16",
            FieldType::Int32 => "i32",
            FieldType::Int64 => "i64",
            FieldType::Uint128 => "u128",
            FieldType::U256 => "U256",
            FieldType::Int128 => "i128",
            FieldType::Float64 => "f64",
            FieldType::Bool => "bool",
            FieldType::String => "String",
            FieldType::Bytes => "Bytes",
            FieldType::AccountId => "AccountId",
            ty => unimplemented!("Unsupported FieldType {ty:?}"),
        }
    }
}

/// The main definition of a service.
///
/// This contains the metadata of the service, the job definitions, and other hooks, along with the
/// gadget that will be executed when one of the jobs is calling this service.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ServiceBlueprint<'a> {
    /// The metadata of the service.
    pub metadata: ServiceMetadata<'a>,
    /// The blueprint manager that will be used to manage the blueprints lifecycle.
    pub manager: BlueprintManager,
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
}

#[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JobMetadata<'a> {
    /// The Job name.
    pub name: BlueprintString<'a>,
    /// The Job description.
    pub description: Option<BlueprintString<'a>>,
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

/// Service Blueprint Manager is a smart contract that will manage the service lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum BlueprintManager {
    /// A Smart contract that will manage the service lifecycle.
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

impl Default for Gadget<'_> {
    fn default() -> Self {
        Gadget::Wasm(WasmGadget {
            runtime: WasmRuntime::Wasmtime,
            sources: vec![],
        })
    }
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TestFetcher<'a> {
    pub cargo_package: BlueprintString<'a>,
    pub cargo_bin: BlueprintString<'a>,
    pub base_path: BlueprintString<'a>,
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
#[serde(untagged)]
enum GadgetSourceFlat<'a> {
    WithFetcher { fetcher: GadgetSourceFetcher<'a> },
    Plain(GadgetSourceFetcher<'a>),
}

impl<'a> From<GadgetSourceFlat<'a>> for GadgetSource<'a> {
    fn from(flat: GadgetSourceFlat<'a>) -> GadgetSource<'a> {
        match flat {
            GadgetSourceFlat::Plain(fetcher) => GadgetSource { fetcher },
            GadgetSourceFlat::WithFetcher { fetcher } => GadgetSource { fetcher },
        }
    }
}

/// A Gadget Source Fetcher is a fetcher that will fetch the gadget
/// from a remote source.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(from = "DynamicGadgetSourceFetcher<'_>")]
pub enum GadgetSourceFetcher<'a> {
    /// A Gadget that will be fetched from the IPFS.
    #[allow(clippy::upper_case_acronyms)]
    IPFS(Vec<u8>),
    /// A Gadget that will be fetched from the Github release.
    Github(GithubFetcher<'a>),
    /// A Gadgets that will be fetched from the container registry.
    ContainerImage(ImageRegistryFetcher<'a>),
    /// For testing
    Testing(TestFetcher<'a>),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum DynamicGadgetSourceFetcher<'a> {
    Tagged(TaggedGadgetSourceFetcher<'a>),
    Untagged(UntaggedGadgetSourceFetcher<'a>),
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
enum TaggedGadgetSourceFetcher<'a> {
    /// A Gadget that will be fetched from the IPFS.
    #[allow(clippy::upper_case_acronyms)]
    IPFS(CidWrapper),
    /// A Gadget that will be fetched from the Github release.
    Github(GithubFetcher<'a>),
    /// A Gadgets that will be fetched from the container registry.
    ContainerImage(ImageRegistryFetcher<'a>),
    /// For testing
    Testing(TestFetcher<'a>),
}

/// A Gadget Source Fetcher is a fetcher that will fetch the gadget
/// from a remote source.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum UntaggedGadgetSourceFetcher<'a> {
    /// A Gadget that will be fetched from the IPFS.
    #[allow(clippy::upper_case_acronyms)]
    IPFS(CidWrapper),
    /// A Gadget that will be fetched from the Github release.
    Github(GithubFetcher<'a>),
    /// A Gadgets that will be fetched from the container registry.
    ContainerImage(ImageRegistryFetcher<'a>),
    /// Testing
    Testing(TestFetcher<'a>),
}

impl<'a> From<DynamicGadgetSourceFetcher<'a>> for GadgetSourceFetcher<'a> {
    fn from(f: DynamicGadgetSourceFetcher<'a>) -> GadgetSourceFetcher<'a> {
        match f {
            DynamicGadgetSourceFetcher::Tagged(tagged) => tagged.into(),
            DynamicGadgetSourceFetcher::Untagged(untagged) => untagged.into(),
        }
    }
}

impl<'a> From<UntaggedGadgetSourceFetcher<'a>> for GadgetSourceFetcher<'a> {
    fn from(untagged: UntaggedGadgetSourceFetcher<'a>) -> GadgetSourceFetcher<'a> {
        match untagged {
            UntaggedGadgetSourceFetcher::IPFS(hash) => GadgetSourceFetcher::IPFS(hash.0.to_bytes()),
            UntaggedGadgetSourceFetcher::Github(fetcher) => GadgetSourceFetcher::Github(fetcher),
            UntaggedGadgetSourceFetcher::ContainerImage(fetcher) => {
                GadgetSourceFetcher::ContainerImage(fetcher)
            }
            UntaggedGadgetSourceFetcher::Testing(fetcher) => GadgetSourceFetcher::Testing(fetcher),
        }
    }
}

impl<'a> From<TaggedGadgetSourceFetcher<'a>> for GadgetSourceFetcher<'a> {
    fn from(tagged: TaggedGadgetSourceFetcher<'a>) -> GadgetSourceFetcher<'a> {
        match tagged {
            TaggedGadgetSourceFetcher::IPFS(hash) => GadgetSourceFetcher::IPFS(hash.0.to_bytes()),
            TaggedGadgetSourceFetcher::Github(fetcher) => GadgetSourceFetcher::Github(fetcher),
            TaggedGadgetSourceFetcher::ContainerImage(fetcher) => {
                GadgetSourceFetcher::ContainerImage(fetcher)
            }
            TaggedGadgetSourceFetcher::Testing(fetcher) => GadgetSourceFetcher::Testing(fetcher),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_blueprint_deserialization() {
        // get the root of the git repo using a command, then make a path using {git repo root}/blueprints/incredible-squaring/blueprint.json
        let process = std::process::Command::new("git")
            .arg("rev-parse")
            .arg("--show-toplevel")
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to run git command");

        let output = String::from_utf8(
            process
                .wait_with_output()
                .expect("Failed to get output of git command")
                .stdout,
        )
        .expect("Failed to convert command output to string");
        let base_path = PathBuf::from(output.trim()).join("blueprints/incredible-squaring/");
        let blueprint_path = base_path.join("blueprint.json");

        let blueprint_content =
            std::fs::read_to_string(blueprint_path).expect("Failed to read blueprint.json");

        let blueprint_content: serde_json::Value = serde_json::from_str(&blueprint_content)
            .expect("Failed to deserialize blueprint.json file");

        // Deserialize the entire Blueprint
        let gadget: Gadget = serde_json::from_str(&blueprint_content["gadget"].to_string())
            .expect("Failed to deserialize blueprint.json");

        // Assertions

        if let Gadget::Native(gadget) = gadget {
            for src in gadget.sources {
                if let GadgetSourceFetcher::Testing(testing) = src.fetcher {
                    assert_eq!(PathBuf::from(testing.base_path.to_string()), base_path);
                    assert_eq!(testing.cargo_bin, "main");
                    assert_eq!(testing.cargo_package, "incredible-squaring-blueprint");
                    return;
                }
            }
        } else {
            panic!("Unexpected Gadget variant");
        }

        panic!(
            "The sources included with the `gadget` field does not have a valid entry for Testing"
        )
    }
}
