use blueprint_sdk::build::blueprint_metadata;

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/main.rs");
    blueprint_metadata::generate_json();
    blueprint_sdk::build::utils::soldeer_install();
    blueprint_sdk::build::utils::soldeer_update();
    blueprint_sdk::build::utils::build_contracts(vec!["contracts"]);
}
