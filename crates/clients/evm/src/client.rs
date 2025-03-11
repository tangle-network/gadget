//! Re-exported from <https://github.com/Layr-Labs/eigensdk-rs/blob/main/crates/chainio/clients/eth/src/client.rs>

use alloy_primitives::BlockNumber;
use alloy_rpc_types::{Block, BlockNumberOrTag};

#[derive(Debug)]
pub struct Client {}

pub trait BackendClient {
    type Error;

    /// Get the latest block number.
    ///
    /// # Returns
    ///
    /// The latest block number.
    async fn block_number(&self) -> Result<BlockNumber, Self::Error>;

    /// Get the block hash given its block number.
    ///
    /// # Arguments
    ///
    /// * `number` - The block number.
    ///
    /// # Returns
    ///
    /// The block having that number.
    async fn block_by_number(&self, number: BlockNumberOrTag)
    -> Result<Option<Block>, Self::Error>;
}
