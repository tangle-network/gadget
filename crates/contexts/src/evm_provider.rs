use gadget_clients::evm::evm_provider::EVMProviderClient;

/// `EVMProviderContext` trait provides access to the EVM provider from the context.
pub trait EVMProviderContext {
    fn client(&self) -> EVMProviderClient;
}
