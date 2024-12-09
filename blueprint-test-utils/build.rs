fn main() {
    let contract_dirs: Vec<&str> = vec!["."];
    blueprint_build_utils::soldeer_update();
    blueprint_build_utils::build_contracts(contract_dirs);
}
