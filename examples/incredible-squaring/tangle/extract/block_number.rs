use blueprint_sdk::{
    __composite_rejection as composite_rejection, __define_rejection as define_rejection,
};
use blueprint_sdk::{job_call::Parts as JobCallParts, FromJobCallParts};

/// Extracts the current block number from the job call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockNumber(pub u32);

impl BlockNumber {
    pub const METADATA_KEY: &'static str = "X-TANGLE-BLOCK-NUMBER";
}

blueprint_sdk::__impl_deref!(BlockNumber: u32);
blueprint_sdk::__impl_from!(u32, BlockNumber);

define_rejection! {
  #[body = "No BlockNumber found in the metadata"]
  /// A Rejection type for [`BlockNumber`] when it is missing from the Metadata.
  pub struct MissingBlockNumber;
}

define_rejection! {
  #[body = "The block number in the metadata is not a valid integer"]
  /// A Rejection type for [`BlockNumber`] when it is not a valid integer.
  pub struct InvalidBlockNumber;
}

composite_rejection! {
    /// Rejection used for [`BlockNumber`].
    ///
    /// Contains one variant for each way the [`Form`](super::Form) extractor
    /// can fail.
    pub enum BlockNumberRejection {
        MissingBlockNumber,
        InvalidBlockNumber,
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
        let block_number_raw = parts
            .metadata
            .get(Self::METADATA_KEY)
            .ok_or(MissingBlockNumber)?;
        let block_number = block_number_raw
            .try_into()
            .map_err(|_| InvalidBlockNumber)?;
        Ok(BlockNumber(block_number))
    }
}
