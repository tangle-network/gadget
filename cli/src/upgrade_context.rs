use alloy_primitives::Address;
use color_eyre::eyre::Result;
use std::path::PathBuf;

/// Build a `TxContext` from the shared `TangleClientArgs` resolver.
pub(super) fn upgrade_tx_ctx(
    network: &cargo_tangle::command::tangle::TangleClientArgs,
) -> Result<cargo_tangle::command::upgrade::TxContext> {
    let cfg = network.client_config(0, None)?;
    Ok(cargo_tangle::command::upgrade::TxContext {
        http_rpc_url: cfg.http_rpc_endpoint.clone(),
        tangle_contract: cfg.settings.tangle_contract,
        keystore_path: PathBuf::from(cfg.keystore_uri.clone()),
    })
}

pub(super) fn upgrade_view_ctx(
    network: &cargo_tangle::command::tangle::TangleClientArgs,
    auditors_contract: Option<Address>,
) -> Result<cargo_tangle::command::upgrade::ViewContext> {
    let cfg = network.client_config(0, None)?;
    Ok(cargo_tangle::command::upgrade::ViewContext {
        http_rpc_url: cfg.http_rpc_endpoint.clone(),
        tangle_contract: cfg.settings.tangle_contract,
        auditors_contract,
    })
}
