use gadget_std::collections::HashMap;
use gadget_std::io::{BufRead, BufReader, Read};
use gadget_std::path::{Path, PathBuf};
use gadget_std::process::{Command, Stdio};

use cargo_metadata::{Metadata, Package};
use gadget_blueprint_proc_macro_core::{
    BlueprintManager, FieldType, Gadget, GadgetSource, GadgetSourceFetcher, JobDefinition,
    MasterBlueprintServiceManagerRevision, NativeGadget, ServiceBlueprint, ServiceMetadata,
    TestFetcher,
};

use rustdoc_types::{
    Crate, Enum, Function, Id, Item, ItemEnum, ItemKind, Module, Struct, StructKind, Type,
    VariantKind,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Blueprint metadata not found in the Cargo.toml")]
    MissingBlueprintMetadata,
    #[error("Failed to deserialize gadget: {0}")]
    DeserializeGadget(serde_json::Error),
    #[error("Unsupported blueprint manager")]
    UnsupportedBlueprintManager,

    #[error("Unit structs are not supported (job: {job}, type: {ty})")]
    UnitStruct { job: String, ty: String },
    #[error("Non-unit enums are not supported (job: {job}, type: {ty})")]
    NonUnitEnum { job: String, ty: String },
    #[error("Unsupported type: {0:?}")]
    UnsupportedType(ItemKind),

    #[error("Failed to generate rustdoc (status: {status:?}, command: {command})")]
    RustdocFailed {
        status: Option<i32>,
        command: String,
    },
}

/// Generate `blueprint.json` to the current crate working directory next to `build.rs` file.
pub fn generate_json() {
    if let Err(e) = Config::builder().build().generate_json() {
        println!("cargo:warning=Failed to generate blueprint metadata: {e}");
        std::process::exit(1);
    }
}

#[derive(Debug, Clone, Default, typed_builder::TypedBuilder)]
pub struct Config {
    /// The output path of the generated `blueprint.json` file.
    #[builder(default, setter(strip_option))]
    output_file: Option<PathBuf>,
    /// The name of the crate to generate the blueprint for, defaults to `CARGO_PKG_NAME`.
    #[builder(default, setter(strip_option))]
    crate_name: Option<String>,
    /// The target directory where `rustdoc` output is stored. Defaults to "[`manifest_dir`](Self::manifest_dir)/target".
    #[builder(default, setter(strip_option))]
    target_dir: Option<PathBuf>,
    /// The  directory where `Cargo.toml` resides. Defaults to `CARGO_MANIFEST_DIR`.
    #[builder(default, setter(strip_option))]
    manifest_dir: Option<PathBuf>,
}

#[derive(Default)]
struct Context {
    current_job: String,
    crates: HashMap<u32, Crate>,
    target_dir: PathBuf,
    manifest_dir: PathBuf,
}

impl Config {
    pub fn generate_json(self) -> Result<(), Error> {
        let output_file = self.output_file.unwrap_or_else(|| {
            std::env::current_dir()
                .expect("Failed to get current directory")
                .join("blueprint.json")
        });

        let crate_name = self.crate_name.unwrap_or_else(|| {
            std::env::var("CARGO_PKG_NAME").expect("Failed to get package name")
        });
        let manifest_dir = self.manifest_dir.unwrap_or_else(|| {
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Failed to get manifest directory")
                .into()
        });
        let target_dir = self
            .target_dir
            .unwrap_or_else(|| manifest_dir.join("target"));

        let krate = generate_rustdoc(&crate_name, &manifest_dir, &target_dir)?;

        // Extract the job definitions from the rustdoc output
        let mut context = Context {
            target_dir,
            manifest_dir,
            ..Default::default()
        };

        let jobs = extract_jobs(&mut context, &krate)?;
        eprintln!("[INFO] Extracted {} job definitions", jobs.len());
        let hooks = extract_hooks(&krate)?;
        let metadata = extract_metadata(&context.manifest_dir)?;
        let package = find_package(&metadata, &crate_name);
        let gadget = generate_gadget(package, &context.manifest_dir)?;
        let metadata = extract_blueprint_metadata(package)?;
        eprintln!("Generating blueprint.json to {:?}", output_file);
        let blueprint = ServiceBlueprint {
            metadata: ServiceMetadata {
                name: crate_name.into(),
                description: std::env::var("CARGO_PKG_DESCRIPTION").map(Into::into).ok(),
                author: std::env::var("CARGO_PKG_AUTHORS").map(Into::into).ok(),
                category: std::env::var("CARGO_PKG_KEYWORDS").map(Into::into).ok(),
                code_repository: std::env::var("CARGO_PKG_REPOSITORY").map(Into::into).ok(),
                logo: None,
                website: std::env::var("CARGO_PKG_HOMEPAGE").map(Into::into).ok(),
                license: std::env::var("CARGO_PKG_LICENSE").map(Into::into).ok(),
            },
            jobs,
            manager: metadata.manager,
            master_manager_revision: metadata.master_blueprint_service_manager_revision,
            registration_params: hooks
                .iter()
                .find_map(|hook| match hook {
                    Hook::RegistrationParams(params) => Some(params.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
            request_params: hooks
                .iter()
                .find_map(|hook| match hook {
                    Hook::RequestParams(params) => Some(params.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
            gadget,
        };

        let json = serde_json::to_string_pretty(&blueprint).expect("Failed to serialize blueprint");
        std::fs::write(&output_file, json).expect("Failed to write blueprint.json");

        Ok(())
    }
}

enum Hook {
    RegistrationParams(Vec<FieldType>),
    RequestParams(Vec<FieldType>),
}

/// Extract hooks from a rustdoc module.
fn extract_hooks(krate: &Crate) -> Result<Vec<Hook>, Error> {
    let root_module = krate
        .index
        .get(&krate.root)
        .expect("Failed to get root module");
    let ItemEnum::Module(blueprint_crate) = &root_module.inner else {
        panic!("Failed to get blueprint crate module");
    };
    extract_hooks_from_module(&krate.root, &krate.index, blueprint_crate)
}

/// Extract job definitions from the rustdoc output.
fn extract_jobs<'a>(
    context: &mut Context,
    krate: &'a Crate,
) -> Result<Vec<JobDefinition<'a>>, Error> {
    let root_module = krate
        .index
        .get(&krate.root)
        .expect("Failed to get root module");
    let ItemEnum::Module(blueprint_crate) = &root_module.inner else {
        panic!("Failed to get blueprint crate module");
    };
    extract_jobs_from_module(context, krate, blueprint_crate)
}

/// Extracts job definitions from a module.
fn extract_jobs_from_module<'a>(
    context: &mut Context,
    krate: &'a Crate,
    module: &'a Module,
) -> Result<Vec<JobDefinition<'a>>, Error> {
    let mut jobs = vec![];
    let automatically_derived: String = String::from("#[automatically_derived]");
    const JOB_DEF: &str = "JOB_DEF";
    for item_id in &module.items {
        let item = krate.index.get(item_id).expect("Failed to get item");
        match &item.inner {
            ItemEnum::Module(m) => {
                jobs.extend(extract_jobs_from_module(context, krate, m)?);
            }
            // Handle only the constant items that are automatically derived and have the JOB_DEF in their name
            ItemEnum::Constant { const_: c, .. }
                if item.attrs.contains(&automatically_derived)
                    && item
                        .name
                        .as_ref()
                        .map(|v| v.contains(JOB_DEF))
                        .unwrap_or(false) =>
            {
                let linked_function_id = item.links.values().next().expect("No linked functions");
                let linked_function = krate
                    .index
                    .get(linked_function_id)
                    .expect("Failed to get linked function");
                let ItemEnum::Function(function) = &linked_function.inner else {
                    panic!("Linked item is not a function");
                };

                let mut contains_non_primitive_types = false;
                if !function.sig.inputs.is_empty() {
                    // Skip the last parameter, which is the context
                    for (_, param_ty) in function
                        .sig
                        .inputs
                        .iter()
                        .take(function.sig.inputs.len() - 1)
                    {
                        if let Type::Primitive(_) = param_ty {
                            continue;
                        }

                        contains_non_primitive_types = true;
                    }
                }

                if !contains_non_primitive_types {
                    contains_non_primitive_types =
                        matches!(&function.sig.output, Some(Type::Primitive(_)) | None);
                }

                let mut job_def: JobDefinition =
                    serde_json::from_str(&unescape_json_string(&c.expr))
                        .expect("Failed to deserialize job definition");

                context.current_job = job_def.metadata.name.to_string();
                if contains_non_primitive_types {
                    extract_non_primitive_parameters(context, &mut job_def, krate, function)?;
                }

                job_def.metadata.description = linked_function.docs.as_ref().map(Into::into);
                jobs.push(job_def);
            }
            _ => continue,
        }
    }

    // Sort jobs by job_id field
    jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));

    Ok(jobs)
}

fn extract_non_primitive_parameters(
    context: &mut Context,
    job_def: &mut JobDefinition,
    krate: &Crate,
    function: &Function,
) -> Result<(), Error> {
    // The function signature will also include the context, so we need to subtract 1
    assert_eq!(job_def.params.len(), function.sig.inputs.len() - 1);

    for (index, (_name, ty)) in function
        .sig
        .inputs
        .iter()
        .enumerate()
        .take(job_def.params.len())
    {
        job_def.params[index] = walk_type(context, ty, krate)?;
    }

    Ok(())
}

fn walk_type(context: &mut Context, ty: &Type, krate: &Crate) -> Result<FieldType, Error> {
    fn on_primitive(name: &str) -> Result<FieldType, Error> {
        match name {
            "bool" => Ok(FieldType::Bool),
            "u8" => Ok(FieldType::Uint8),
            "u16" => Ok(FieldType::Uint16),
            "u32" => Ok(FieldType::Uint32),
            "u64" => Ok(FieldType::Uint64),
            "u128" => Ok(FieldType::Uint128),
            "i8" => Ok(FieldType::Int8),
            "i16" => Ok(FieldType::Int16),
            "i32" => Ok(FieldType::Int32),
            "i64" => Ok(FieldType::Int64),
            "i128" => Ok(FieldType::Int128),
            "f32" | "f64" => Ok(FieldType::Float64),
            "char" => todo!("char"),
            _ => panic!("Unexpected primitive type"),
        }
    }

    match ty {
        Type::ResolvedPath(path) => {
            let Some(qualified_path) = krate.paths.get(&path.id) else {
                panic!("Failed to get qualified path");
            };

            match qualified_path.kind {
                ItemKind::Struct | ItemKind::Enum | ItemKind::Primitive => {}
                kind => return Err(Error::UnsupportedType(kind)),
            }

            let krate_to_check = if qualified_path.crate_id == 0 {
                krate
            } else {
                let external_krate = krate
                    .external_crates
                    .get(&qualified_path.crate_id)
                    .expect("Failed to get crate");
                context
                    .crates
                    .entry(qualified_path.crate_id)
                    .or_insert_with(|| {
                        generate_rustdoc(
                            &external_krate.name,
                            &context.manifest_dir,
                            &context.target_dir,
                        )
                        .unwrap()
                    });

                todo!("lookup crate");
            };

            let item = krate_to_check
                .index
                .get(&path.id)
                .expect("Failed to get struct");

            match &item.inner {
                ItemEnum::Struct(s) => {
                    walk_struct(context, item.name.as_ref().unwrap(), s, krate_to_check)
                }
                ItemEnum::Enum(e) => {
                    verify_enum(context, item.name.as_ref().unwrap(), e, krate_to_check)?;
                    Ok(FieldType::String)
                }
                ItemEnum::Primitive(p) => on_primitive(&p.name),
                _ => unreachable!("Should only have supported types at this point"),
            }
        }

        Type::Primitive(primitive) => on_primitive(primitive),
        Type::Tuple(_) => todo!("tuple types"),
        Type::Array { .. } => todo!("array types"),
        Type::QualifiedPath { .. } => todo!("qualified path types"),
        _ => panic!("Unexpected type"),
    }
}

fn walk_struct(
    context: &mut Context,
    name: &str,
    s: &Struct,
    krate: &Crate,
) -> Result<FieldType, Error> {
    match &s.kind {
        StructKind::Unit => Err(Error::UnitStruct {
            job: context.current_job.clone(),
            ty: name.to_string(),
        }),
        StructKind::Tuple(fields) => {
            let mut resolved_fields = Vec::with_capacity(fields.len());
            for field in fields {
                let field = field.unwrap();
                let struct_field_item = krate.index.get(&field).expect("Failed to get field");
                let ItemEnum::StructField(struct_field_ty) = &struct_field_item.inner else {
                    panic!("Expected struct field")
                };

                resolved_fields.push((
                    struct_field_item.name.clone().unwrap(),
                    Box::new(walk_type(context, struct_field_ty, krate)?),
                ));
            }

            Ok(FieldType::Struct(name.to_string(), resolved_fields))
        }
        StructKind::Plain {
            fields,
            has_stripped_fields,
        } => {
            assert!(!has_stripped_fields);

            let mut resolved_fields = Vec::with_capacity(fields.len());
            for field in fields {
                let struct_field_item = krate.index.get(field).expect("Failed to get field");
                let ItemEnum::StructField(struct_field_ty) = &struct_field_item.inner else {
                    panic!("Expected struct field")
                };

                resolved_fields.push((
                    struct_field_item.name.clone().unwrap(),
                    Box::new(walk_type(context, struct_field_ty, krate)?),
                ));
            }

            Ok(FieldType::Struct(name.to_string(), resolved_fields))
        }
    }
}

fn verify_enum(context: &Context, name: &str, e: &Enum, krate: &Crate) -> Result<(), Error> {
    for variant in &e.variants {
        let variant_item = krate.index.get(variant).expect("Failed to get variant");
        let ItemEnum::Variant(variant_ty) = &variant_item.inner else {
            panic!("Expected variant")
        };

        if variant_ty.kind != VariantKind::Plain {
            return Err(Error::NonUnitEnum {
                job: context.current_job.clone(),
                ty: name.to_string(),
            });
        }
    }

    Ok(())
}

/// Extracts hooks from a module.
fn extract_hooks_from_module(
    _root: &Id,
    index: &HashMap<Id, Item>,
    module: &Module,
) -> Result<Vec<Hook>, Error> {
    let mut hooks = vec![];
    let automatically_derived: String = String::from("#[automatically_derived]");
    const REGISTRATION_HOOK_PARAMS: &str = "REGISTRATION_HOOK_PARAMS";
    const REQUEST_HOOK_PARAMS: &str = "REQUEST_HOOK_PARAMS";

    for item_id in &module.items {
        let item = index.get(item_id).expect("Failed to get item");
        match &item.inner {
            ItemEnum::Module(m) => {
                hooks.extend(extract_hooks_from_module(_root, index, m)?);
            }
            ItemEnum::Constant { const_: c, .. }
                if item.attrs.contains(&automatically_derived)
                    && item
                        .name
                        .as_ref()
                        .map(|v| v.eq(REQUEST_HOOK_PARAMS))
                        .unwrap_or(false) =>
            {
                let value = serde_json::from_str(&unescape_json_string(&c.expr))
                    .expect("Failed to deserialize hook");
                hooks.push(Hook::RequestParams(value));
            }

            ItemEnum::Constant { const_: c, .. }
                if item.attrs.contains(&automatically_derived)
                    && item
                        .name
                        .as_ref()
                        .map(|v| v.eq(REGISTRATION_HOOK_PARAMS))
                        .unwrap_or(false) =>
            {
                let value = serde_json::from_str(&unescape_json_string(&c.expr))
                    .expect("Failed to deserialize hook");
                hooks.push(Hook::RegistrationParams(value));
            }
            _ => continue,
        }
    }
    Ok(hooks)
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

/// Finds a package in the workspace to get metadata for.
fn find_package<'m>(
    metadata: &'m cargo_metadata::Metadata,
    pkg_name: &str,
) -> &'m cargo_metadata::Package {
    assert!(
        !metadata.workspace_members.is_empty(),
        "There should be at least one package in the workspace"
    );

    metadata
        .packages
        .iter()
        .find(|p| p.name == pkg_name)
        .expect("No package found in the workspace with the specified name")
}

struct Locked;

struct LockFile {
    file: std::fs::File,
}

impl LockFile {
    fn new(base_path: &Path) -> Result<Self, Error> {
        std::fs::create_dir_all(base_path).expect("Failed to create lock file directory");
        let path = base_path.join("blueprint.lock");
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .expect("Failed to create lock file");
        Ok(Self { file })
    }

    fn try_lock(&self) -> Result<(), Locked> {
        use fs2::FileExt;
        self.file.try_lock_exclusive().map_err(|_| Locked)
    }
}

impl Drop for LockFile {
    #[allow(unstable_name_collisions)]
    fn drop(&mut self) {
        // Unlock the file
        use fs2::FileExt;
        let _ = self.file.unlock();
    }
}

fn extract_metadata(manifest_dir: &Path) -> Result<Metadata, Error> {
    let root = Path::new(&manifest_dir)
        .canonicalize()
        .expect("Failed to canonicalize root dir");

    let lock = LockFile::new(&root)?;
    if lock.try_lock().is_err() {
        eprintln!("Already locked; skipping rustdoc generation",);
        // Exit early if the lock file exists
        std::process::exit(0);
    }
    let toml_file = root.join("Cargo.toml");
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(&toml_file)
        .no_deps()
        .exec()
        .expect("Failed to get metadata");
    Ok(metadata)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BlueprintMetadata {
    manager: BlueprintManager,
    #[serde(alias = "master_revision", default)]
    master_blueprint_service_manager_revision: MasterBlueprintServiceManagerRevision,
}

fn extract_blueprint_metadata(package: &Package) -> Result<BlueprintMetadata, Error> {
    let Some(blueprint) = package.metadata.get("blueprint") else {
        eprintln!("[ERROR]: No blueprint metadata found in the Cargo.toml.");
        eprintln!("[ERROR]: For more information, see: <TODO>");
        return Err(Error::MissingBlueprintMetadata);
    };

    let mut metadata: BlueprintMetadata =
        serde_json::from_value(blueprint.clone()).map_err(Error::DeserializeGadget)?;
    match &mut metadata.manager {
        BlueprintManager::Evm(manager) => {
            let path = resolve_evm_contract_path_by_name(manager);
            *manager = path.display().to_string();
        }
        _ => return Err(Error::UnsupportedBlueprintManager),
    };

    Ok(metadata)
}

/// Generates the metadata for the gadget.
fn generate_gadget(package: &Package, manifest_dir: &Path) -> Result<Gadget<'static>, Error> {
    let root = Path::new(&manifest_dir)
        .canonicalize()
        .expect("Failed to canonicalize root dir");
    let mut sources = vec![];
    if let Some(gadget) = package.metadata.get("gadget") {
        let gadget: Gadget<'static> =
            serde_json::from_value(gadget.clone()).expect("Failed to deserialize gadget.");
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
        matches!(
            fetcher,
            GadgetSource {
                fetcher: GadgetSourceFetcher::Testing(..)
            }
        )
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

fn generate_rustdoc(
    crate_name: &str,
    manifest_dir: &Path,
    target_dir: &Path,
) -> Result<Crate, Error> {
    let root = std::path::Path::new(&manifest_dir);
    let lock = LockFile::new(root)?;
    if lock.try_lock().is_err() {
        eprintln!("Already locked; skipping rustdoc generation",);
        // Exit early if the lock file exists
        std::process::exit(0);
    }
    let mut cmd = Command::new("cargo");

    cmd.arg("--quiet")
        .arg("rustdoc")
        .args(["-Z", "unstable-options"])
        .args(["--output-format", "json"])
        .args(["--package", crate_name])
        .arg("--lib")
        .args(["--target-dir", &target_dir.to_string_lossy()])
        .arg("--locked")
        .args(["--", "--document-hidden-items"])
        .env("RUSTC_BOOTSTRAP", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let final_cmd = format!("{cmd:?}");
    let mut child = cmd
        .spawn()
        .unwrap_or_else(|err| panic!("{err} while spawning command: {final_cmd}"));

    enum StdoutOrStderr {
        Stdout,
        Stderr,
    }

    fn collect_output(kind: StdoutOrStderr, r: impl Read) -> Vec<String> {
        let r = BufReader::new(r);
        let mut lines = Vec::new();
        for l in r.lines() {
            let l = l.unwrap();
            match kind {
                StdoutOrStderr::Stdout => println!("{l}"),
                StdoutOrStderr::Stderr => eprintln!("{l}"),
            }
            lines.push(l)
        }
        lines
    }

    let stdout = std::thread::spawn({
        let r = child.stdout.take().unwrap();
        move || collect_output(StdoutOrStderr::Stdout, r)
    });
    let stderr = std::thread::spawn({
        let r = child.stderr.take().unwrap();
        move || collect_output(StdoutOrStderr::Stderr, r)
    });

    let status = child
        .wait()
        .unwrap_or_else(|err| panic!("{err} while waiting for command: {final_cmd}"));
    if !status.success() {
        eprintln!("command failed: {final_cmd}");
        eprintln!("=== stdout");
        let stdout = stdout.join().unwrap();
        for line in stdout {
            eprintln!("{line}");
        }
        eprintln!("=== stderr");
        let stderr = stderr.join().unwrap();
        for line in stderr {
            eprintln!("{line}");
        }
        eprintln!("===");
        return Err(Error::RustdocFailed {
            status: status.code(),
            command: final_cmd,
        });
    }

    let crate_name_snake_case = kabab_case_to_snake_case(crate_name);
    let json_path = format!("{}/doc/{crate_name_snake_case}.json", target_dir.display());
    eprintln!("Reading JSON from {json_path}");
    let json_string = std::fs::read_to_string(&json_path).expect("Failed to read rustdoc JSON");
    let krate: Crate = serde_json::from_str(&json_string).expect("Failed to parse rustdoc JSON");
    assert!(
        krate.format_version >= 33,
        "This tool expects JSON format version >= 33",
    );

    Ok(krate)
}

fn kabab_case_to_snake_case(s: &str) -> String {
    s.replace('-', "_")
}

/// A simple function to unscape JSON strings that are escaped multiple times
fn unescape_json_string(s: &str) -> String {
    let mut s = s.to_string();
    while let Ok(unescape) = serde_json::from_str::<String>(&s) {
        s = unescape;
    }
    s
}
