use gadget_clients::evm::evm_provider::EVMProviderClient;

/// `EVMProviderContext` trait provides access to the EVM provider from the context.
pub trait EVMProviderContext {
    type Network: alloy_network::Network;
    type Transport: alloy_transport::Transport + Clone;
    type Provider: alloy_provider::Provider<Self::Transport, Self::Network>;

    fn client<N: alloy_network::Network, T: alloy_transport::Transport, P: alloy_provider::Provider<T, N>>(&self) -> EVMProviderClient<N, T, P>;
}