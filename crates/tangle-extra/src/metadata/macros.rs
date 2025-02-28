use super::types::blueprint::{BlueprintServiceManager, ServiceBlueprint, ServiceMetadata};
use crate::metadata::types::gadget::{Gadget, GadgetSource, GadgetSourceFetcher, NativeGadget, TestFetcher};
use crate::metadata::types::job::JobDefinition;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::service::MasterBlueprintServiceManagerRevision;

#[doc(hidden)]
pub mod ext {
    pub use serde_json;
    pub use tangle_subxt;
    pub use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::FieldType;
    pub use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::jobs::JobDefinition as SubxtJobDefinition;
    pub use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::types::MembershipModelType;
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No manager contract specified (ex. `manager: \"MyBlueprint\"`)")]
    NoManager,
    #[error("Unsupported blueprint service manager")]
    UnsupportedBlueprintServiceManager,
    #[error(
        "Unable to determine package name, `name` field not specified in macro, and `CARGO_PKG_NAME` is unset"
    )]
    FetchPackageName,
    #[error("Unable to find the current package: `{0}`")]
    PackageNotFound(String),
    #[error("Expected environment variable `{0}` to be set")]
    MissingEnvVar(&'static str),

    #[error("Failed to de/serialize value: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    CargoMetadata(#[from] cargo_metadata::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[doc(hidden)]
#[derive(Deserialize, Debug)]
pub struct PartialBlueprintJson {
    name: Option<String>,
    description: Option<String>,
    author: Option<String>,
    category: Option<String>,
    code_repository: Option<String>,
    logo: Option<String>,
    website: Option<String>,
    license: Option<String>,
    jobs: Option<Vec<ext::SubxtJobDefinition>>,
    registration_params: Option<Vec<ext::FieldType>>,
    request_params: Option<Vec<ext::FieldType>>,
    manager: Option<BlueprintServiceManager>,
    master_manager_revision: Option<MasterBlueprintServiceManagerRevision>,
    #[expect(dead_code, reason = "Not yet used in blueprint.json")]
    supported_membership_models: Option<Vec<ext::MembershipModelType>>,
}

/// Resolves the path to the EVM contract JSON file by its name.
///
/// The contracts are expected to be in the `contracts/out` directory.
fn resolve_evm_contract_path_by_name(name: &str) -> PathBuf {
    PathBuf::from("contracts")
        .join("out")
        .join(format!("{name}.sol"))
        .join(format!("{name}.json"))
}

impl PartialBlueprintJson {
    pub fn finalize(self) -> Result<ServiceBlueprint<'static>, Error> {
        let Some(mut manager) = self.manager else {
            return Err(Error::NoManager);
        };

        #[allow(unreachable_patterns)]
        match &mut manager {
            BlueprintServiceManager::Evm(manager) => {
                let path = resolve_evm_contract_path_by_name(manager);
                *manager = path.display().to_string();
            }
            _ => return Err(Error::UnsupportedBlueprintServiceManager),
        }

        let jobs = self
            .jobs
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(job_id, job)| {
                let mut job: JobDefinition = job.into();
                job.job_id = job_id as u64;
                job
            })
            .collect();

        let name = match self.name {
            Some(name) => name,
            None => std::env::var("CARGO_PKG_NAME").map_err(|_| Error::FetchPackageName)?,
        };

        Ok(ServiceBlueprint {
            metadata: ServiceMetadata {
                name: name.into(),
                description: self
                    .description
                    .or_else(|| std::env::var("CARGO_PKG_DESCRIPTION").ok())
                    .map(Into::into),
                author: self
                    .author
                    .or_else(|| std::env::var("CARGO_PKG_AUTHORS").ok())
                    .map(Into::into),
                category: self
                    .category
                    .or_else(|| std::env::var("CARGO_PKG_KEYWORDS").ok())
                    .map(Into::into),
                code_repository: self
                    .code_repository
                    .or_else(|| std::env::var("CARGO_PKG_REPOSITORY").ok())
                    .map(Into::into),
                logo: self.logo.map(Into::into),
                website: self
                    .website
                    .or_else(|| std::env::var("CARGO_PKG_HOMEPAGE").ok())
                    .map(Into::into),
                license: self
                    .license
                    .or_else(|| std::env::var("CARGO_PKG_LICENSE").ok())
                    .map(Into::into),
            },
            jobs,
            registration_params: self.registration_params.unwrap_or_default(),
            request_params: self.request_params.unwrap_or_default(),
            manager,
            master_manager_revision: self
                .master_manager_revision
                .unwrap_or(MasterBlueprintServiceManagerRevision::Latest),
            gadget: generate_gadget_for_current_crate()?,
        })
    }
}

/// Generates the metadata for the gadget.
fn generate_gadget_for_current_crate() -> Result<Gadget<'static>, Error> {
    let root = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| Error::MissingEnvVar("CARGO_MANIFEST_DIR"))?;
    let root = Path::new(&root).canonicalize()?;

    let toml_file = root.join("Cargo.toml");

    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(&toml_file)
        .no_deps()
        .exec()?;

    let package_name =
        std::env::var("CARGO_PKG_NAME").map_err(|_| Error::MissingEnvVar("CARGO_PKG_NAME"))?;
    let package = metadata
        .packages
        .iter()
        .find(|p| p.name == package_name)
        .ok_or(Error::PackageNotFound(package_name))?;

    let mut sources = vec![];
    if let Some(gadget) = package.metadata.get("gadget") {
        let gadget: Gadget<'static> = serde_json::from_value(gadget.clone())?;
        if let Gadget::Native(NativeGadget { sources: fetchers }) = gadget {
            sources.extend(fetchers);
        } else {
            panic!("Currently unsupported gadget type has been parsed")
        }
    } else {
        eprintln!("[WARN] No gadget metadata found in the Cargo.toml.");
        eprintln!("[WARN] For more information, see: <TODO>");
    };

    let has_test_fetcher = sources.iter().any(|fetcher| {
        matches!(fetcher, GadgetSource {
            fetcher: GadgetSourceFetcher::Testing(..)
        })
    });

    if !has_test_fetcher {
        println!("Adding test fetcher since none exists");
        sources.push(GadgetSource {
            fetcher: GadgetSourceFetcher::Testing(TestFetcher {
                cargo_package: package.name.clone().into(),
                cargo_bin: "main".into(),
                base_path: format!("{}", root.display()).into(),
            }),
        })
    }

    assert_ne!(sources.len(), 0, "No sources found for the gadget");
    Ok(Gadget::Native(NativeGadget { sources }))
}

/// Define a Tangle blueprint.json file
///
/// ## Usage
///
/// ```rust
/// use blueprint_tangle_extra::blueprint;
///
/// # async fn foo() {}
/// # async fn bar() {}
///
/// let blueprint = blueprint! {
///     name: "MyBlueprint",
///     description: "A blueprint that does cool things",
///     author: "Foo",
///     license: "MIT",
///     jobs: [foo, bar]
/// };
///
/// println!("{blueprint:?}");
/// ```
///
/// ## Fields
///
/// ### Basic Text
/// * `name`: The name of the blueprint, defaults to env `CARGO_PKG_NAME`
/// * `description`: The description of the blueprint, defaults to env `CARGO_PKG_DESCRIPTION`
/// * `author`: The author of the blueprint, defaults to env `CARGO_PKG_AUTHORS`
/// * `category`: The category of the blueprint, defaults to env `CARGO_PKG_KEYWORDS`
/// * `code_repository`: The code repository of the blueprint, defaults to env `CARGO_PKG_REPOSITORY`
/// * `logo`: A URL to the logo representing this blueprint, defaults to `None`
/// * `website`: A URL to the website representing this blueprint, defaults to env `CARGO_PKG_HOMEPAGE`
/// * `license`: The license this blueprint falls under, defaults to env `CARGO_PKG_LICENSE`
///
/// ### Variable Arrays
/// * `jobs`: A list of job names, imported from the blueprint library.
/// * `registration_params`: A list of [`FieldType`] variants, indicating the arguments necessary for registration
/// * `request_params`: A list of [`FieldType`] variants, indicating the arguments necessary for requesting a service
/// * `manager`: The name of the smart contract that will manage the service (likely the UpperCamelCase version of your crate name)
/// * `master_manager_revision`: The revision of the Master Blueprint Service Manager (MBSM), as a string
/// * `supported_membership_models`: A list of the supported [`MembershipModelType`] variants
#[macro_export]
macro_rules! blueprint {
    ($($tt:tt)*) => {{
        let mut object = $crate::metadata::macros::ext::serde_json::Map::new();
        $crate::blueprint_inner!(@__CONSTRUCT object $($tt)*);
        match $crate::metadata::macros::ext::serde_json::from_value::<
            $crate::metadata::macros::PartialBlueprintJson,
        >(object.into())
        {
            ::core::result::Result::Ok(partial_json) => partial_json.finalize(),
            ::core::result::Result::Err(e) => {
                ::core::result::Result::Err($crate::metadata::macros::Error::Serialization(e))
            }
        }
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! blueprint_inner {
    // The two possible base cases

    // Input ended without a trailing comma
    (@__CONSTRUCT $object:ident ()) => {};
    // Input ended with a trailing comma
    (@__CONSTRUCT $object:ident) => {};

    // Handle the last field being defined without a trailing comma (just inject one).
    (@__CONSTRUCT $object:ident $field:ident: { $variant:ident = $value:expr }) => {
        $crate::blueprint_inner!(@__CONSTRUCT $object $field: { $variant = $value } , ())
    };
    (@__CONSTRUCT $object:ident $field:ident: [$($items:ident),* $(,)?]) => {
        $crate::blueprint_inner!(@__CONSTRUCT $object $field: [$($items),*] , ())
    };
    (@__CONSTRUCT $object:ident $field:ident: $value:expr) => {
        $crate::blueprint_inner!(@__CONSTRUCT $object $field: $value , ())
    };

    // Basic text fields
    (@__CONSTRUCT $object:ident name: $name:expr , $($rest:tt)*) => {
        $object.insert(
            String::from("name"),
            $crate::metadata::macros::ext::serde_json::Value::String(String::from($name))
        );
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };
    (@__CONSTRUCT $object:ident description: $description:expr , $($rest:tt)*) => {
        $object.insert(
            String::from("description"),
            $crate::metadata::macros::ext::serde_json::Value::String(String::from($description))
        );
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };
    (@__CONSTRUCT $object:ident author: $name:expr , $($rest:tt)*) => {
        $object.insert(
            String::from("author"),
            $crate::metadata::macros::ext::serde_json::Value::String(String::from($name))
        );
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };
    (@__CONSTRUCT $object:ident category: $category:expr , $($rest:tt)*) => {
        $object.insert(
            String::from("category"),
            $crate::metadata::macros::ext::serde_json::Value::String(String::from($category))
        );
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };
    (@__CONSTRUCT $object:ident code_repository: $code_repository:expr , $($rest:tt)*) => {
        $object.insert(
            String::from("code_repository"),
            $crate::metadata::macros::ext::serde_json::Value::String(String::from($code_repository))
        );
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };
    (@__CONSTRUCT $object:ident logo: $logo:expr , $($rest:tt)*) => {
        $object.insert(
            String::from("logo"),
            $crate::metadata::macros::ext::serde_json::Value::String(String::from($logo))
        );
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };
    (@__CONSTRUCT $object:ident website: $website:expr , $($rest:tt)*) => {
        $object.insert(
            String::from("website"),
            $crate::metadata::macros::ext::serde_json::Value::String(String::from($website))
        );
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };
    (@__CONSTRUCT $object:ident license: $license:expr , $($rest:tt)*) => {
        $object.insert(
            String::from("license"),
            $crate::metadata::macros::ext::serde_json::Value::String(String::from($license))
        );
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };

    (@__CONSTRUCT $object:ident jobs: [$($job_name:ident),* $(,)?] , $($rest:tt)*) => {{
        #[allow(unused_mut, unused_imports)]
        {
            use $crate::metadata::IntoJobDefinition;

            let mut jobs = Vec::new();
            $(
                let definition: $crate::metadata::macros::ext::SubxtJobDefinition = $job_name.into_job_definition();
                jobs.push($crate::metadata::macros::ext::serde_json::to_value(definition).expect("should serialize"));
            )*

            $object.insert(
                String::from("jobs"),
                $crate::metadata::macros::ext::serde_json::Value::Array(jobs)
            );
        }
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*);
    }};
    (@__CONSTRUCT $object:ident registration_params: [$($registration_params:expr),* $(,)?] , $($rest:tt)*) => {
        // TODO: Generate registration params
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };
    (@__CONSTRUCT $object:ident request_params: [$($request_params:expr),* $(,)?] , $($rest:tt)*) => {
        // TODO: Generate request params
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };
    (@__CONSTRUCT $object:ident manager: { $variant:ident = $value:literal } , $($rest:tt)*) => {{
        let mut manager_obj = $crate::metadata::macros::ext::serde_json::Map::new();
        manager_obj.insert(
            String::from(stringify!($variant)),
            $crate::metadata::macros::ext::serde_json::Value::String(String::from($value))
        );

        $object.insert(
            String::from("manager"),
            $crate::metadata::macros::ext::serde_json::Value::Object(manager_obj)
        );
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*);
    }};
    (@__CONSTRUCT $object:ident master_manager_revision: $master_manager_revision:expr , $($rest:tt)*) => {
        $object.insert(
            String::from("master_manager_revision"),
            $crate::metadata::macros::ext::serde_json::Value::String(String::from($master_manager_revision))
        );
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };
    (@__CONSTRUCT $object:ident supported_membership_models: [$($membership_models:expr),* $(,)?] , $($rest:tt)*) => {
        // TODO: Generate supported membership models
        $crate::blueprint_inner!(@__CONSTRUCT $object $($rest)*)
    };

    // Error on invalid fields
    (@__CONSTRUCT $object:ident $unknown:ident: $value:expr , $($rest:tt)*) => {
        panic!("Unknown or invalid field `{}`", stringify!($unknown));
    };
}

#[cfg(test)]
mod tests {
    use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::FieldType;
    use super::*;
    use crate::extract::{TangleArg, TangleResult};
    use crate::metadata::types::job::JobMetadata;

    #[test]
    fn info_test() {
        let blueprint = blueprint! {
            name: "test",
            description: "test blueprint",
            author: "Someone",
            category: "testing",
            code_repository: "https://example.com",
            logo: "https://example.com/logo",
            website: "https://example.com",
            license: "MIT",
            master_manager_revision: "Latest",
            manager: { Evm = "TestBlueprint" }
        }
        .unwrap();

        assert_eq!(blueprint.metadata, ServiceMetadata {
            name: "test".into(),
            description: Some("test blueprint".into()),
            author: Some("Someone".into()),
            category: Some("testing".into()),
            code_repository: Some("https://example.com".into()),
            logo: Some("https://example.com/logo".into()),
            website: Some("https://example.com".into()),
            license: Some("MIT".into()),
        });
        assert_eq!(
            blueprint.manager,
            BlueprintServiceManager::Evm(String::from(
                "contracts/out/TestBlueprint.sol/TestBlueprint.json"
            ))
        );
        assert_eq!(
            blueprint.master_manager_revision,
            MasterBlueprintServiceManagerRevision::Latest
        );
        assert_eq!(blueprint.jobs, Vec::new());
        assert_eq!(blueprint.registration_params, Vec::new());
        assert_eq!(blueprint.request_params, Vec::new());
    }

    #[test]
    fn with_jobs() {
        async fn foo() -> TangleResult<u64> {
            TangleResult(0)
        }
        async fn bar(TangleArg(_): TangleArg<u64>) -> TangleResult<u64> {
            TangleResult(0)
        }

        let blueprint = blueprint! {
            name: "test",
            master_manager_revision: "Latest",
            manager: { Evm = "TestBlueprint" },
            jobs: [foo, bar]
        }
        .unwrap();

        assert_eq!(blueprint.jobs, vec![
            JobDefinition {
                job_id: 0,
                metadata: JobMetadata {
                    name: "blueprint_tangle_extra::metadata::macros::tests::with_jobs::foo".into(),
                    description: None,
                },
                params: vec![],
                result: vec![FieldType::Uint64],
            },
            JobDefinition {
                job_id: 1,
                metadata: JobMetadata {
                    name: "blueprint_tangle_extra::metadata::macros::tests::with_jobs::bar".into(),
                    description: None,
                },
                params: vec![FieldType::Uint64],
                result: vec![FieldType::Uint64],
            }
        ])
    }

    #[test]
    fn generates_testing_gadget() {
        let blueprint = blueprint! {
            name: "test",
            manager: { Evm = "TestBlueprint" }
        }
        .unwrap();

        let Gadget::Native(NativeGadget { sources }) = blueprint.gadget else {
            panic!("Not a native gadget");
        };

        let Some(testing_gadget) = sources.first() else {
            panic!("No sources defined");
        };

        let GadgetSourceFetcher::Testing(fetcher) = &testing_gadget.fetcher else {
            panic!("Not a testing fetcher");
        };

        assert_eq!(fetcher.cargo_package, "blueprint-tangle-extra");
        assert_eq!(fetcher.cargo_bin, "main");
    }

    #[test]
    fn trailing_commas_special_cases() {
        let _ = blueprint! {
            name: "test",
            manager: { Evm = "TestBlueprint" },
        };

        let _ = blueprint! {
            name: "test",
            jobs: [],
        };
    }

    #[test]
    fn no_trailing_commas_special_cases() {
        let _ = blueprint! {
            name: "test",
            manager: { Evm = "TestBlueprint" }
        };

        let _ = blueprint! {
            name: "test",
            jobs: []
        };
    }
}
