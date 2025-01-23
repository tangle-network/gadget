pub mod error;
pub mod tangle;

#[cfg(test)]
mod tests {
    use crate::tangle::{get_client, TangleConfig};
    use gadget_config::protocol::TangleInstanceSettings;
    use gadget_config::{GadgetConfiguration, ProtocolSettings};
    use gadget_runner_core::config::BlueprintConfig;

    fn create_test_config() -> GadgetConfiguration {
        let mut config = GadgetConfiguration::default();
        config.http_rpc_endpoint = "http://localhost:9933".to_string();
        config.ws_rpc_endpoint = "ws://localhost:9944".to_string();
        config.protocol_settings = ProtocolSettings::Tangle(TangleInstanceSettings::default());
        config
    }

    #[tokio::test]
    async fn test_get_client() {
        let result = get_client("ws://localhost:9944", "http://localhost:9933").await;
        // This test will fail if there's no local node running, which is expected
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_requires_registration_with_mock_config() {
        let config = TangleConfig::default();
        let env = create_test_config();

        let result = config.requires_registration(&env).await;
        // This will fail without a running node, which is expected
        assert!(result.is_err());
    }
}
