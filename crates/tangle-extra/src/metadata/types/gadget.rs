use alloc::borrow::Cow;

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
    pub owner: Cow<'a, str>,
    /// The repository name.
    pub repo: Cow<'a, str>,
    /// The release tag of the repository.
    /// NOTE: The tag should be a valid semver tag.
    pub tag: Cow<'a, str>,
    /// The names of the binary in the release by the arch and the os.
    pub binaries: Vec<GadgetBinary<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TestFetcher<'a> {
    pub cargo_package: Cow<'a, str>,
    pub cargo_bin: Cow<'a, str>,
    pub base_path: Cow<'a, str>,
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
    /// Amd64 architecture (`x86_64`).
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
    /// `MacOS` operating system.
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
    pub name: Cow<'a, str>,
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
            GadgetSourceFlat::Plain(fetcher) | GadgetSourceFlat::WithFetcher { fetcher } => {
                GadgetSource { fetcher }
            }
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
    registry: Cow<'a, str>,
    /// The name of the image.
    image: Cow<'a, str>,
    /// The tag of the image.
    tag: Cow<'a, str>,
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
    /// The WASM binary will be executed using the `WASMtime` runtime.
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
