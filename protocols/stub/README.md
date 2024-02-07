# Creating a new protocol

#### Step 1: Add `gadget-common`, `async-trait`, `protocol-macros`, `tangle-primitives`, and `tokio` as a dependencies
```toml
gadget-common = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true }
tangle-primitives = { workspace = true }
protocol-macros = { workspace = true }
```

#### Step 2: Create a Config struct as such:
```rust
#[protocol]
pub struct StubConfig<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId, MaxParticipants, MaxSubmissionLen, MaxKeyLen, MaxDataLen, MaxSignatureLen, MaxProofLen>,
{
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
    client: C,
    _pd: std::marker::PhantomData<(B, BE)>,
}
```

#### Step 3: Create a `StubNetwork` struct, and implement `Network` and `Clone` for it

#### Step 4: Create a `StubProtocol` struct, and implement `GadgetProtocol` and `AsyncProtocol` for it

#### Step 5: Implement `NetworkAndProtocolSetup` for `StubConfig`, passing in `StubNetwork` as the network and `StubProtocol` as the protocol

#### Step 6: Running the protocol
First, create an instance of the `StubConfig` struct, then, call `execute` on it
```rust
pub async fn run<B: Block, BE: Backend<B> + 'static, C: ClientWithApi<B, BE>>(
    client: C,
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
) -> Result<(), Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId, MaxParticipants, MaxSubmissionLen, MaxKeyLen, MaxDataLen, MaxSignatureLen, MaxProofLen>,
{
    let config = StubConfig {
        pallet_tx,
        logger,
        client,
        _pd: std::marker::PhantomData,
    };

    config.execute().await
}
```