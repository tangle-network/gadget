fn main() {
    println!("cargo:rerun-if-changed=src/cli");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/*");
    blueprint_metadata::generate_json();
}
