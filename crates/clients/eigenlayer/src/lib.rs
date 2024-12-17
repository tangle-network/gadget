pub mod eigenlayer;
pub mod error;

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, U256};
    use alloy_provider::Provider;
    use eigenlayer::EigenlayerClient;
    use gadget_anvil_utils::{start_anvil_container, ANVIL_STATE_PATH};
    use gadget_config::{
        load,
        protocol::{EigenlayerContractAddresses, Protocol, ProtocolSettings},
        supported_chains::SupportedChains,
        ContextConfig, GadgetCLICoreSettings, GadgetConfiguration,
    };

    async fn setup_test_environment() -> (String, String, GadgetConfiguration) {
        let (_container, http_endpoint, ws_endpoint) =
            start_anvil_container(ANVIL_STATE_PATH, false).await;

        // Create test configuration
        let context_config = ContextConfig::create_eigenlayer_config(
            http_endpoint.parse().unwrap(),
            ws_endpoint.parse().unwrap(),
            String::new(),
            None,
            SupportedChains::LocalTestnet,
            EigenlayerContractAddresses::default(),
        );
        let config = load(context_config).unwrap();

        (http_endpoint, ws_endpoint, config)
    }

    #[tokio::test]
    async fn test_get_provider_http() {
        let (_, _, config) = setup_test_environment().await;
        let client = EigenlayerClient { config };
        let provider = client.get_provider_http();
        assert!(provider.get_block_number().await.is_ok());
    }

    #[tokio::test]
    async fn test_get_provider_ws() {
        let (_, _, config) = setup_test_environment().await;
        let client = EigenlayerClient { config };
        let provider = client.get_provider_ws().await.unwrap();
        assert!(provider.get_block_number().await.is_ok());
    }

    #[tokio::test]
    async fn test_get_slasher_address() {
        let (_, _, config) = setup_test_environment().await;
        let client = EigenlayerClient { config };
        let delegation_manager_addr = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
        let result = client.get_slasher_address(delegation_manager_addr).await;
        assert!(result.is_err()); // Will error since contract doesn't exist in test environment
    }

    #[tokio::test]
    async fn test_avs_registry_reader() {
        let (_, _, config) = setup_test_environment().await;
        let client = EigenlayerClient { config };
        let result = client.avs_registry_reader().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_avs_registry_writer() {
        let (_, _, config) = setup_test_environment().await;
        let client = EigenlayerClient { config };
        let private_key = "0000000000000000000000000000000000000000000000000000000000000001";
        let result = client.avs_registry_writer(private_key.to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_operator_info_service() {
        let (_, _, config) = setup_test_environment().await;
        let client = EigenlayerClient { config };
        let result = client.operator_info_service_in_memory().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_operator_stake_in_quorums() {
        let (_, _, config) = setup_test_environment().await;
        let client = EigenlayerClient { config };
        let result = client
            .get_operator_stake_in_quorums_at_block(1, vec![1, 2].into())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_operator_id() {
        let (_, _, config) = setup_test_environment().await;
        let client = EigenlayerClient { config };
        let operator_addr = address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92266");
        let result = client.get_operator_id(operator_addr).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_operator_details() {
        let (_, _, config) = setup_test_environment().await;
        let client = EigenlayerClient { config };
        let operator_addr = address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92266");
        let result = client.get_operator_details(operator_addr).await;
        assert!(result.is_err()); // Will error since contract doesn't exist in test environment
    }
}
