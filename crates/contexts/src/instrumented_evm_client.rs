pub use gadget_clients::evm::instrumented_client::InstrumentedClient;

/// `EvmInstrumentedClientContext` trait provides access to the EVM provider from the context.
#[async_trait::async_trait]
pub trait EvmInstrumentedClientContext {
    async fn evm_client(&self) -> InstrumentedClient;
}
