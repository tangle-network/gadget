//! Block extractors for EVM
//!
//! Simple extractors for block-related data from EVM.

use alloy_primitives::B256;
use blueprint_core::{
    __composite_rejection as composite_rejection, __define_rejection as define_rejection,
    __impl_deref as impl_deref, FromJobCallParts, job_call::Parts as JobCallParts,
};

/// Block number extractor
#[derive(Debug, Clone, Copy)]
pub struct BlockNumber(pub u64);

impl BlockNumber {
    /// Metadata key for block number
    pub const METADATA_KEY: &'static str = "X-EVM-BLOCK-NUMBER";
}

impl_deref!(BlockNumber: u64);

define_rejection! {
    #[body = "No block found in metadata"]
    /// This rejection is used to indicate that a block was not found in the metadata.
    pub struct MissingBlock;
}

define_rejection! {
    #[body = "Block number must be a valid number"]
    /// This rejection is used to indicate that the block number is not a valid number.
    pub struct InvalidBlockNumber;
}

define_rejection! {
    #[body = "Block number must be a valid block hash"]
    /// This rejection is used to indicate that the block hash is not a valid hash.
    pub struct InvalidBlockHash;
}

composite_rejection! {
    /// Rejection for block-related extractors
    pub enum BlockNumberRejection {
        MissingBlock,
        InvalidBlockNumber,
    }
}

impl TryFrom<&mut JobCallParts> for BlockNumber {
    type Error = BlockNumberRejection;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let block_number = parts
            .metadata
            .get(Self::METADATA_KEY)
            .ok_or(MissingBlock)?
            .try_into()
            .map_err(|_| InvalidBlockNumber)?;
        Ok(BlockNumber(block_number))
    }
}

impl<Ctx> FromJobCallParts<Ctx> for BlockNumber
where
    Ctx: Send + Sync,
{
    type Rejection = BlockNumberRejection;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}

composite_rejection! {
    /// Rejection for block-related extractors
    pub enum BlockHashRejection {
        MissingBlock,
        InvalidBlockHash,
    }
}

/// Block hash extractor
#[derive(Debug, Clone, Copy)]
pub struct BlockHash(pub B256);

impl BlockHash {
    /// Metadata key for block hash
    pub const METADATA_KEY: &'static str = "X-EVM-BLOCK-HASH";
}

impl_deref!(BlockHash: B256);

impl TryFrom<&mut JobCallParts> for BlockHash {
    type Error = BlockHashRejection;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let block_hash = parts
            .metadata
            .get(Self::METADATA_KEY)
            .ok_or(MissingBlock)?
            .as_bytes();

        let hash = B256::try_from(block_hash).map_err(|_| InvalidBlockHash)?;
        Ok(BlockHash(hash))
    }
}

impl<Ctx> FromJobCallParts<Ctx> for BlockHash
where
    Ctx: Send + Sync,
{
    type Rejection = BlockHashRejection;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}

/// Block timestamp extractor
#[derive(Debug, Clone, Copy)]
pub struct BlockTimestamp(pub u64);

impl BlockTimestamp {
    /// Metadata key for block timestamp
    pub const METADATA_KEY: &'static str = "X-EVM-BLOCK-TIMESTAMP";
}

impl_deref!(BlockTimestamp: u64);

define_rejection! {
    #[body = "Block timestamp must be a valid block timestamp"]
    /// This rejection is used to indicate that the block timestamp is not a valid timestamp.
    pub struct InvalidBlockTimestamp;
}

composite_rejection! {
    /// Rejection for block-related extractors
    pub enum BlockTimestampRejection {
        MissingBlock,
        InvalidBlockTimestamp,
    }
}

impl TryFrom<&mut JobCallParts> for BlockTimestamp {
    type Error = BlockTimestampRejection;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let timestamp = parts
            .metadata
            .get(Self::METADATA_KEY)
            .ok_or(MissingBlock)?
            .try_into()
            .map_err(|_| InvalidBlockTimestamp)?;
        Ok(BlockTimestamp(timestamp))
    }
}

impl<Ctx> FromJobCallParts<Ctx> for BlockTimestamp
where
    Ctx: Send + Sync,
{
    type Rejection = BlockTimestampRejection;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}
