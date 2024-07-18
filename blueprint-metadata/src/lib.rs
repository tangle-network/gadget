use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
};

use gadget_blueprint_proc_macro_core::{
    Gadget, ServiceBlueprint, ServiceMetadata, ServiceRegistrationHook, ServiceRequestHook,
    WasmGadget, WasmRuntime,
};

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct Config {
    /// The output path of the generated `blueprint.json` file.
    #[builder(default, setter(strip_option))]
    output_file: Option<std::path::PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self { output_file: None }
    }
}

impl Config {
    pub fn generate_json(self) {
        let output_file = self.output_file.unwrap_or_else(|| {
            std::env::current_dir()
                .expect("Failed to get current directory")
                .join("blueprint.json")
        });
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
            jobs: vec![],
            registration_hook: ServiceRegistrationHook::None,
            registration_params: vec![],
            request_hook: ServiceRequestHook::None,
            request_params: vec![],
            gadget: Gadget::Wasm(WasmGadget {
                runtime: WasmRuntime::Wasmtime,
                soruces: vec![],
            }),
        };

        let json = serde_json::to_string_pretty(&blueprint).expect("Failed to serialize blueprint");
        std::fs::write(&output_file, json).expect("Failed to write blueprint.json");
        generate_rustdoc();
    }
}

/// Generate `blueprint.json` to the current crate working directory next to `build.rs` file.
pub fn generate_json() {
    Config::builder().build().generate_json();
}

fn generate_rustdoc() {
    let crate_name = std::env::var("CARGO_PKG_NAME").expect("Failed to get package name");
    let target_dir = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("target");
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

    let json_path = format!("{custom_target_dir}/doc/{crate_name}.json");
    let json_payload = std::fs::read(json_path).unwrap();
    let krate: rustdoc_types::Crate = serde_json::from_slice(&json_payload).unwrap();
    assert!(
        krate.format_version >= 28,
        "This tool expects JSON format version 28",
    );
    panic!("krate: {krate:#?}");
}
