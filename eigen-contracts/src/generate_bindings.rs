mod tests {

    use ethers::prelude::Abigen;

    /// Generate rust bindings using ethers
    fn generate_bindings(contract_name: &str, input_path: &str, output_path: &str) {
        let coontract: String = format!("../bindings/utils/json/{input_path}").to_string();

        match Abigen::new(&contract_name, coontract) {
            Ok(v) => {
                let _ = v
                    .generate()
                    .expect("failed to abigen")
                    .write_to_file(format!("{output_path}/src/{contract_name}.rs"));
            }
            Err(e) => {
                println!("abigenerr{}", e);
            }
        }
    }

    #[test]
    fn test_binding_generation() {
        generate_bindings(
            "RegistryCoordinator",
            "RegistryCoordinator.json",
            "../bindings",
        );
        generate_bindings(
            "OperatorStateRetriever",
            "OperatorStateRetriever.json",
            "../bindings",
        );
        generate_bindings("StakeRegistry", "StakeRegistry.json", "../bindings");
        generate_bindings("BLSApkRegistry", "BLSApkRegistry.json", "../bindings");
        generate_bindings(
            "ServiceManagerBase",
            "ServiceManagerBase.json",
            "../bindings",
        );
        generate_bindings("DelegationManager", "DelegationManager.json", "../bindings");
        generate_bindings("StrategyManager", "StrategyManager.json", "../bindings");
        generate_bindings("AVSDirectory", "AVSDirectory.json", "../bindings");
        generate_bindings("ISlasher", "ISlasher.json", "../bindings");
        generate_bindings("IStrategy", "IStrategy.json", "../bindings");
        generate_bindings("IERC20", "IERC20.json", "../bindings");
    }
}
