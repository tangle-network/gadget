fn main() {
    println!("cargo:rerun-if-changed=src/main.rs");
    blueprint_metadata::generate_json();
}
