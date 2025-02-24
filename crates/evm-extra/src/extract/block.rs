//! Block extractors for EVM
//!
//! Simple extractors for block-related data from EVM.

use alloy_primitives::B256;
use alloy_rpc_types::{Block, Transaction};
use blueprint_core::{
    __define_rejection as define_rejection, __impl_deref as impl_deref, FromJobCallParts,
    job_call::Parts as JobCallParts,
};

/// Block number extractor
#[derive(Debug, Clone, Copy)]
pub struct BlockNumber(pub u64);

impl_deref!(BlockNumber: u64);

define_rejection! {
    #[body = "No block found in extensions"]
    /// This rejection is used to indicate that a block was not found in the extensions.
    pub struct MissingBlock;
}

impl TryFrom<&mut JobCallParts> for BlockNumber {
    type Error = MissingBlock;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let block = parts
            .extensions
            .get::<Block<Transaction>>()
            .ok_or(MissingBlock)?;
        Ok(BlockNumber(block.header.number))
    }
}

impl<Ctx> FromJobCallParts<Ctx> for BlockNumber
where
    Ctx: Send + Sync,
{
    type Rejection = MissingBlock;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}

/// Block hash extractor
#[derive(Debug, Clone, Copy)]
pub struct BlockHash(pub B256);

impl_deref!(BlockHash: B256);

impl TryFrom<&mut JobCallParts> for BlockHash {
    type Error = MissingBlock;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let block = parts
            .extensions
            .get::<Block<Transaction>>()
            .ok_or(MissingBlock)?;
        Ok(BlockHash(block.header.hash))
    }
}

impl<Ctx> FromJobCallParts<Ctx> for BlockHash
where
    Ctx: Send + Sync,
{
    type Rejection = MissingBlock;

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

impl_deref!(BlockTimestamp: u64);

impl TryFrom<&mut JobCallParts> for BlockTimestamp {
    type Error = MissingBlock;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let block = parts
            .extensions
            .get::<Block<Transaction>>()
            .ok_or(MissingBlock)?;
        Ok(BlockTimestamp(block.header.timestamp))
    }
}

impl<Ctx> FromJobCallParts<Ctx> for BlockTimestamp
where
    Ctx: Send + Sync,
{
    type Rejection = MissingBlock;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}
