use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use gadget_blueprint_proc_macro_core::{
    FieldType, Gadget, JobDefinition, JobResultVerifier, ServiceBlueprint, ServiceMetadata,
    ServiceRegistrationHook, ServiceRequestHook,
};

use rustdoc_types::{Crate, Id, Item, ItemEnum, Module};

/// Generate `blueprint.json` to the current crate working directory next to `build.rs` file.
pub fn generate_json() {
    Config::builder().build().generate_json();
}

#[derive(Debug, Clone, Default, typed_builder::TypedBuilder)]
pub struct Config {
    /// The output path of the generated `blueprint.json` file.
    #[builder(default, setter(strip_option))]
    output_file: Option<std::path::PathBuf>,
}

impl Config {
    pub fn generate_json(self) {
        let output_file = self.output_file.unwrap_or_else(|| {
            std::env::current_dir()
                .expect("Failed to get current directory")
                .join("blueprint.json")
        });
        let krate = generate_rustdoc();
        // Extract the job definitions from the rustdoc output
        let jobs = extract_jobs(&krate);
        let hooks = extract_hooks(&krate);
        let gadget = extract_gadget(&output_file);

        eprintln!("Generating blueprint.json to {:?}", output_file);
        let blueprint = ServiceBlueprint {
            metadata: ServiceMetadata {
                name: std::env::var("CARGO_PKG_NAME")
                    .expect("Failed to get package name")
                    .into(),
                description: std::env::var("CARGO_PKG_DESCRIPTION").map(Into::into).ok(),
                author: std::env::var("CARGO_PKG_AUTHORS").map(Into::into).ok(),
                category: std::env::var("CARGO_PKG_KEYWORDS").map(Into::into).ok(),
                code_repository: std::env::var("CARGO_PKG_REPOSITORY").map(Into::into).ok(),
                logo: None,
                website: std::env::var("CARGO_PKG_HOMEPAGE").map(Into::into).ok(),
                license: std::env::var("CARGO_PKG_LICENSE").map(Into::into).ok(),
            },
            jobs,
            registration_hook: hooks
                .iter()
                .find_map(|hook| match hook {
                    Hook::Registration(hook) => Some(hook.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
            registration_params: hooks
                .iter()
                .find_map(|hook| match hook {
                    Hook::RegistrationParams(params) => Some(params.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
            request_hook: hooks
                .iter()
                .find_map(|hook| match hook {
                    Hook::Request(hook) => Some(hook.clone()),
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
    }
}

enum Hook {
    Registration(ServiceRegistrationHook),
    RegistrationParams(Vec<FieldType>),
    Request(ServiceRequestHook),
    RequestParams(Vec<FieldType>),
}

/// Extract hooks from a rustdoc module.
fn extract_hooks(krate: &Crate) -> Vec<Hook> {
    let root_module = krate
        .index
        .get(&krate.root)
        .expect("Failed to get root module");
    let ItemEnum::Module(blueprint_crate) = &root_module.inner else {
        panic!("Failed to get blueprint crate module");
    };
    extract_hooks_from_module(&krate.root, &krate.index, blueprint_crate)
}

/// Extract the `gadget` field from the rustdoc output.
fn extract_gadget<'a, T: AsRef<Path> + 'a>(blueprint_json_path: T) -> Gadget<'a> {
    let json_string = unescape_json_string(
        &std::fs::read_to_string(blueprint_json_path).expect("Failed to read blueprint.json file"),
    );
    let mut blueprint_json: serde_json::Value =
        serde_json::from_str(&json_string).expect("Failed to parse blueprint JSON");

    if let serde_json::Value::Object(map) = &blueprint_json {
        if map.contains_key("gadget") {
            serde_json::from_value(blueprint_json["gadget"].take())
                .expect("Failed to deserialize gadget JSON")
        } else {
            panic!("The blueprint.json file does not contain a `gadget` field")
        }
    } else {
        panic!("The blueprint.json file is not an OBJECT")
    }
}

/// Extract job definitions from the rustdoc output.
fn extract_jobs(krate: &Crate) -> Vec<JobDefinition<'_>> {
    let root_module = krate
        .index
        .get(&krate.root)
        .expect("Failed to get root module");
    let ItemEnum::Module(blueprint_crate) = &root_module.inner else {
        panic!("Failed to get blueprint crate module");
    };
    extract_jobs_from_module(&krate.root, &krate.index, blueprint_crate)
}

/// Extracts job definitions from a module.
fn extract_jobs_from_module<'a>(
    _root: &'a Id,
    index: &'a HashMap<Id, Item>,
    module: &'a Module,
) -> Vec<JobDefinition<'a>> {
    let mut jobs = vec![];
    let automatically_derived: String = String::from("#[automatically_derived]");
    const JOB_DEF: &str = "JOB_DEF";
    for item_id in &module.items {
        let item = index.get(item_id).expect("Failed to get item");
        match &item.inner {
            ItemEnum::Module(m) => {
                jobs.extend(extract_jobs_from_module(_root, index, m));
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
                let linked_function = index
                    .get(linked_function_id)
                    .expect("Failed to get linked function");
                assert!(
                    matches!(linked_function.inner, ItemEnum::Function(_)),
                    "Linked item is not a function"
                );
                let mut job_def: JobDefinition =
                    serde_json::from_str(&unescape_json_string(&c.expr))
                        .expect("Failed to deserialize job definition");
                job_def.metadata.description = linked_function.docs.as_ref().map(Into::into);
                if let JobResultVerifier::Evm(c) = &mut job_def.verifier {
                    *c = resolve_evm_contract_path_by_name(c).display().to_string();
                }
                jobs.push(job_def);
            }
            _ => continue,
        }
    }
    jobs
}

/// Extracts hooks from a module.
fn extract_hooks_from_module(_root: &Id, index: &HashMap<Id, Item>, module: &Module) -> Vec<Hook> {
    let mut hooks = vec![];
    let automatically_derived: String = String::from("#[automatically_derived]");
    const REGISTRATION_HOOK: &str = "REGISTRATION_HOOK";
    const REQUEST_HOOK: &str = "REQUEST_HOOK";
    const REGISTRATION_HOOK_PARAMS: &str = "REGISTRATION_HOOK_PARAMS";
    const REQUEST_HOOK_PARAMS: &str = "REQUEST_HOOK_PARAMS";

    for item_id in &module.items {
        let item = index.get(item_id).expect("Failed to get item");
        match &item.inner {
            ItemEnum::Module(m) => {
                hooks.extend(extract_hooks_from_module(_root, index, m));
            }
            ItemEnum::Constant { const_: c, .. }
                if item.attrs.contains(&automatically_derived)
                    && item
                        .name
                        .as_ref()
                        .map(|v| v.eq(REGISTRATION_HOOK))
                        .unwrap_or(false) =>
            {
                let mut value = serde_json::from_str(&unescape_json_string(&c.expr))
                    .expect("Failed to deserialize hook");
                if let ServiceRegistrationHook::Evm(c) = &mut value {
                    *c = resolve_evm_contract_path_by_name(c).display().to_string();
                }
                hooks.push(Hook::Registration(value));
            }

            ItemEnum::Constant { const_: c, .. }
                if item.attrs.contains(&automatically_derived)
                    && item
                        .name
                        .as_ref()
                        .map(|v| v.eq(REQUEST_HOOK))
                        .unwrap_or(false) =>
            {
                let mut value = serde_json::from_str(&unescape_json_string(&c.expr))
                    .expect("Failed to deserialize hook");
                if let ServiceRequestHook::Evm(c) = &mut value {
                    *c = resolve_evm_contract_path_by_name(c).display().to_string();
                }
                hooks.push(Hook::Request(value));
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
    hooks
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

struct Locked;

struct LockFile {
    file: std::fs::File,
}

impl LockFile {
    fn new(base_path: &Path) -> Self {
        std::fs::create_dir_all(base_path).expect("Failed to create lock file directory");
        let path = base_path.join("blueprint.lock");
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .expect("Failed to create lock file");
        Self { file }
    }

    fn try_lock(&self) -> Result<(), Locked> {
        use fs2::FileExt;
        self.file.try_lock_exclusive().map_err(|_| Locked)
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        // Unlock the file
        use fs2::FileExt;
        let _ = self.file.unlock();
    }
}

fn generate_rustdoc() -> Crate {
    let root = std::env::var("CARGO_MANIFEST_DIR").expect("Failed to get manifest directory");
    let root = std::path::Path::new(&root);
    let crate_name = std::env::var("CARGO_PKG_NAME").expect("Failed to get package name");
    let target_dir = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("target");
    let lock = LockFile::new(root);
    if lock.try_lock().is_err() {
        eprintln!("Already locked; skipping rustdoc generation",);
        // Exit early if the lock file exists
        std::process::exit(0);
    }
    let custom_target_dir = format!("{}/blueprint", target_dir.display());
    let mut cmd = Command::new("cargo");
    cmd.arg("rustdoc");
    cmd.args(["-Z", "unstable-options"]);
    cmd.args(["--output-format", "json"]);
    cmd.args(["--package", &crate_name]);
    cmd.args(["--target-dir", &custom_target_dir]);
    cmd.arg("--locked");
    cmd.env("RUSTC_BOOTSTRAP", "1");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
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
        panic!("command returned status {status:?}, command was: {final_cmd}")
    }

    let crate_name_snake_case = kabab_case_to_snake_case(&crate_name);
    let json_path = format!("{custom_target_dir}/doc/{crate_name_snake_case}.json");
    eprintln!("Reading JSON from {json_path}");
    let json_string = std::fs::read_to_string(&json_path).expect("Failed to read rustdoc JSON");
    let krate: Crate = serde_json::from_str(&json_string).expect("Failed to parse rustdoc JSON");
    assert!(
        krate.format_version >= 30,
        "This tool expects JSON format version >= 30",
    );
    krate
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
