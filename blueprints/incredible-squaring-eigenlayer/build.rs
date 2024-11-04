fn main() {
    let contract_dirs: Vec<&str> = vec![
        "./contracts/lib/eigenlayer-middleware/lib/eigenlayer-contracts",
        "./contracts/lib/eigenlayer-middleware",
        "./contracts/lib/forge-std",
        "./contracts",
    ];

    blueprint_build_utils::build_contracts(contract_dirs);
}
