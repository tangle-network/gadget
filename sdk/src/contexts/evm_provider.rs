use std::future::Future;

/// `EVMProviderContext` trait provides access to the EVM provider from the context.
pub trait EVMProviderContext {
    type Network: alloy_network::Network;
    type Transport: alloy_transport::Transport + Clone;
    type Provider: alloy_provider::Provider<Self::Transport, Self::Network>;
    /// Get the EVM provider from the context.
    fn evm_provider(
        &self,
    ) -> impl Future<Output = color_eyre::Result<Self::Provider, alloy_transport::TransportError>>;
}
