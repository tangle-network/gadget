pub mod command;
pub mod foundry;
pub mod utils;

#[cfg(test)]
mod tests;

use blueprint_tangle_extra::util::TxProgressExt;
pub use gadget_chain_setup::anvil;
use tangle_subxt::subxt::{Config, blocks::ExtrinsicEvents, client::OnlineClientT, tx::TxProgress};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::BoundedString;

pub(crate) async fn wait_for_in_block_success<T: Config>(
    res: TxProgress<T, impl OnlineClientT<T>>,
) -> ExtrinsicEvents<T> {
    res.wait_for_in_block()
        .await
        .unwrap()
        .fetch_events()
        .await
        .unwrap()
}

/// Helper function to decode a `BoundedString` to a regular String
pub(crate) fn decode_bounded_string(bounded_string: &BoundedString) -> String {
    String::from_utf8_lossy(&bounded_string.0.0).to_string()
}
