use gadget_client_evm::instrumented_client::InstrumentedClient;

/// `EvmInstrumentedClientContext` trait provides access to the EVM provider from the context.
pub trait EvmInstrumentedClientContext {
    fn client(&self) -> InstrumentedClient;
}
