use blueprint_sdk::{
    __composite_rejection as composite_rejection, __define_rejection as define_rejection,
};
use blueprint_sdk::{job_call::Parts as JobCallParts, FromJobCallParts};
use tangle_subxt::subxt::utils::H256;

/// Extracts the current block hash from the job call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockHash(pub H256);

impl BlockHash {
    pub const METADATA_KEY: &'static str = "X-TANGLE-BLOCK-HASH";
}

blueprint_sdk::__impl_deref!(BlockHash: H256);
blueprint_sdk::__impl_from!(H256, BlockHash);

define_rejection! {
  #[body = "No BlockHash found in the metadata"]
  /// A Rejection type for [`BlockHash`] when it is missing from the Metadata.
  pub struct MissingBlockHash;
}

define_rejection! {
  #[body = "The block hash in the metadata is not a valid 32 bytes"]
  /// A Rejection type for [`BlockHash`] when it is not a valid 32 bytes.
  pub struct InvalidBlockHash;
}

composite_rejection! {
    /// Rejection used for [`BlockHash`].
    ///
    /// Contains one variant for each way the [`BlockHash`] extractor
    /// can fail.
    pub enum BlockHashRejection {
        MissingBlockHash,
        InvalidBlockHash,
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
        let block_hash_raw = parts
            .metadata
            .get(Self::METADATA_KEY)
            .ok_or(MissingBlockHash)?;
        let block_hash = block_hash_raw.as_bytes();
        if block_hash.len() != 32 {
            return Err(InvalidBlockHash.into());
        }
        Ok(BlockHash(H256::from_slice(&block_hash)))
    }
}
