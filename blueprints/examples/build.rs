fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/main.rs");
    blueprint_sdk::build::utils::build_contracts(vec!["contracts"]);
    blueprint_sdk::build::blueprint_metadata::generate_json();
}
