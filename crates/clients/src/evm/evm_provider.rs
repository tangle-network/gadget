use alloy_network::Ethereum;
use alloy_provider::{Provider, RootProvider};
use alloy_transport::BoxTransport;
use gadget_config::GadgetConfiguration;
use std::future::Future;

pub struct EVMProviderClient {
    pub config: GadgetConfiguration,
}

impl EVMProviderClient {
    pub fn evm_provider(
        &self,
    ) -> impl Future<Output = Result<RootProvider<BoxTransport, Ethereum>, Error>> {
        static PROVIDER: std::sync::OnceLock<RootProvider<BoxTransport, Ethereum>> =
            std::sync::OnceLock::new();
        async {
            match PROVIDER.get() {
                Some(provider) => Ok(provider.clone()),
                None => {
                    let rpc_url = self.config.http_rpc_endpoint.clone();
                    let provider = alloy_provider::ProviderBuilder::new()
                        .with_recommended_fillers()
                        .on_builtin(&rpc_url)
                        .await
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
                        .root();
                    PROVIDER
                        .set(provider.clone())
                        .map(|_| provider.clone())
                        .map_err(|_| {
                            std::io::Error::new(std::io::ErrorKind::Other, "Failed to set provider")
                        })
                }
            }
        }
    }
}
