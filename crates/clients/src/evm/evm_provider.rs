use gadget_config::GadgetConfiguration;
use std::future::Future;

pub struct EVMProviderClient<
    N: alloy_network::Network,
    T: alloy_transport::Transport,
    P: alloy_provider::Provider<T, N>,
> {
    pub config: GadgetConfiguration,
}

impl<N, T, P> EVMProviderClient<N, T, P> {
    pub fn evm_provider(&self) -> impl Future<Output = Result<P, std::io::Error>> {
        static PROVIDER: std::sync::OnceLock<P> = std::sync::OnceLock::new();
        async {
            match PROVIDER.get() {
                Some(provider) => Ok(provider.clone()),
                None => {
                    let rpc_url = self.config.http_rpc_endpoint.clone();
                    let provider = alloy_provider::ProviderBuilder::new()
                        .with_recommended_fillers()
                        .on_builtin(&rpc_url)
                        .await
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                    PROVIDER
                        .set(provider.clone())
                        .map(|_| provider)
                        .map_err(|_| {
                            std::io::Error::new(std::io::ErrorKind::Other, "Failed to set provider")
                        })
                }
            }
        }
    }
}
