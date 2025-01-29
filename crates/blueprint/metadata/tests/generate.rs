use blueprint_metadata::Config;
use gadget_blueprint_proc_macro_core::{FieldType, ServiceBlueprint};
use std::env::set_current_dir;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;

const MANIFEST_META: &str = r#"
[package.metadata.blueprint]
manager = { Evm = "DoesntMatter" }
master_revision = "Latest"
"#;

fn do_test(asset_name: &str) -> ServiceBlueprint {
    const PACKAGE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

    let temp = tempfile::tempdir().unwrap();
    let metadata_output = temp.path().join("blueprint.json");

    set_current_dir(temp.path()).unwrap();

    let out = Command::new("cargo")
        .args(["init", "--lib", "--name", asset_name])
        .output()
        .unwrap();

    if !out.status.success() {
        panic!("Failed to create new package: {:?}", out);
    }

    // Copy over the gadget Cargo.lock
    std::fs::copy(
        Path::new(PACKAGE_ROOT)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Cargo.lock"),
        temp.path().join("Cargo.lock"),
    )
    .unwrap();

    // Add blueprint-sdk to the dependencies, with all features
    let sdk_path = Path::new(PACKAGE_ROOT)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("sdk");
    let out = Command::new("cargo")
        .args(["add", "blueprint-sdk", "--path"])
        .arg(sdk_path)
        .args(["--features", "std,tangle,evm,eigenlayer,macros"])
        .output()
        .unwrap();

    if !out.status.success() {
        panic!("Failed to add blueprint-sdk: {:?}", out);
    }

    // TODO: Hack
    /////////
    let out = Command::new("cargo")
        .args(["add", "parity-scale-codec@3.6.12"])
        .output()
        .unwrap();

    if !out.status.success() {
        panic!("Failed to add parity-scale-codec: {:?}", out);
    }
    //////////

    let src_file_path = temp.path().join("src").join("lib.rs");
    let mut src_file = OpenOptions::new()
        .truncate(true)
        .write(true)
        .open(src_file_path)
        .unwrap();

    // Copy over the asset file
    let asset_file_name = format!("{}.rs", asset_name);
    let asset_contents = std::fs::read_to_string(
        Path::new(PACKAGE_ROOT)
            .join("tests")
            .join("assets")
            .join(asset_file_name),
    )
    .unwrap();
    src_file.write_all(asset_contents.as_bytes()).unwrap();

    // Add metadata to the manifest
    let mut manifest = OpenOptions::new()
        .append(true)
        .write(true)
        .open(temp.path().join("Cargo.toml"))
        .unwrap();
    manifest.write_all(MANIFEST_META.as_bytes()).unwrap();

    if let Err(e) = Config::builder()
        .manifest_dir(temp.path().to_path_buf())
        .target_dir(temp.path().join("target"))
        .crate_name(asset_name.to_string())
        .output_file(metadata_output.clone())
        .build()
        .generate_json()
    {
        panic!("Failed to generate metadata: {:?}", e);
    }

    serde_json::from_str(&std::fs::read_to_string(metadata_output).unwrap()).unwrap()
}

#[test]
fn generate_job_with_primitive_type_params() {
    let blueprint = do_test("job_primitive_params");
    assert_eq!(blueprint.jobs.len(), 1);

    let job = &blueprint.jobs[0];
    assert_eq!(job.job_id, 0);
    assert_eq!(job.metadata.name, "xsquare");

    assert_eq!(job.params.len(), 1);
    assert_eq!(job.params[0].as_rust_type(), "u64");
}

#[test]
fn generate_job_with_primitive_struct_param() {
    let blueprint = do_test("job_primitive_struct_param");
    assert_eq!(blueprint.jobs.len(), 1);

    let job = &blueprint.jobs[0];
    assert_eq!(job.job_id, 0);
    assert_eq!(job.metadata.name, "xsquare");

    assert_eq!(job.params.len(), 1);
    assert_eq!(job.params[0].as_rust_type(), "SomeParam");
    assert_eq!(
        job.params[0],
        FieldType::Struct(
            "SomeParam".to_string(),
            vec![
                (String::from("a"), Box::new(FieldType::Uint8),),
                (String::from("b"), Box::new(FieldType::Uint16),),
                (String::from("c"), Box::new(FieldType::Uint32),),
                (String::from("d"), Box::new(FieldType::Uint64),),
                (String::from("e"), Box::new(FieldType::Uint128),),
                (String::from("f"), Box::new(FieldType::Int8),),
                (String::from("g"), Box::new(FieldType::Int16),),
                (String::from("h"), Box::new(FieldType::Int32),),
                (String::from("i"), Box::new(FieldType::Int64),),
                (String::from("j"), Box::new(FieldType::Int128),),
                (String::from("k"), Box::new(FieldType::Float64),),
                (String::from("l"), Box::new(FieldType::Float64),),
                (String::from("m"), Box::new(FieldType::Bool),),
            ]
        )
    );
}
