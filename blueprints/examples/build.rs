fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=contracts/src/*");
    blueprint_build_utils::build_contracts(vec!["contracts"]);
    blueprint_metadata::generate_json();
}
