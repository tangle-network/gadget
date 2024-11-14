fn main() {
    let contract_dirs: Vec<&str> = vec![
        "./contracts/lib/core",
        "./contracts/lib/forge-std",
        "./contracts",
    ];
    blueprint_build_utils::build_contracts(contract_dirs);
}
