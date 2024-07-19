fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    blueprint_metadata::generate_json();
}
