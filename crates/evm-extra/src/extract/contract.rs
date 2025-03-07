//! Contract Address Extractor.
//!
//! This simple extractor is used to extract the contract address from the current job call.
//! This could be useful when you are using the [`MatchesContract`] filter.
//!
//! [`MatchesContract`]: crate::filters::MatchesContract

use alloy_primitives::Address;

use blueprint_core::{
    __composite_rejection as composite_rejection, __define_rejection as define_rejection,
    __impl_deref as impl_deref, __impl_from as impl_from, FromJobCallParts,
    job_call::Parts as JobCallParts,
};

/// Extracts the contract address from the current job call.
#[derive(Debug, Clone, Copy)]
pub struct ContractAddress(pub Address);

impl ContractAddress {
    /// The unique key used to store the contract address in the metadata.
    pub const METADATA_KEY: &'static str = "X-EVM-CONTRACT-ADDRESS";
}

impl_deref!(ContractAddress: Address);
impl_from!(Address, ContractAddress);

define_rejection! {
    #[body = "No contract address found in the metadata"]
    /// This rejection is used to indicate that no contract address was found in the metadata.
    pub struct MissingContractAddress;
}

define_rejection! {
    #[body = "Contract address must be a valid address"]
    /// This rejection is used to indicate that the contract address is not a valid address.
    pub struct InvalidContractAddress;
}

composite_rejection! {
    /// Rejection for contract address extractor
    pub enum ContractAddressRejection {
        MissingContractAddress,
        InvalidContractAddress,
    }
}

impl TryFrom<&mut JobCallParts> for ContractAddress {
    type Error = ContractAddressRejection;

    fn try_from(parts: &mut JobCallParts) -> Result<Self, Self::Error> {
        let address = parts
            .metadata
            .get(Self::METADATA_KEY)
            .ok_or(MissingContractAddress)?;
        let address_bytes = address.as_bytes();
        let address = Address::try_from(address_bytes).map_err(|_| InvalidContractAddress)?;
        Ok(ContractAddress(address))
    }
}

impl<Ctx> FromJobCallParts<Ctx> for ContractAddress
where
    Ctx: Send + Sync,
{
    type Rejection = ContractAddressRejection;
    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        Self::try_from(parts)
    }
}
