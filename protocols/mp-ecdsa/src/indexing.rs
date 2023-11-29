use sp_application_crypto::codec;
use std::fmt;

/// A Keygen Party Id, in the range [1, n]
///
/// This is a wrapper around u16 to ensure that the party id is in the range [1, n] and to prevent
/// the misuse of the party id as an offline party id, for example.
///
/// To construct a KeygenPartyId, use the `try_from` method.
#[derive(
Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, codec::Encode, codec::Decode,
)]
#[cfg_attr(feature = "scale-info", derive(scale_info::TypeInfo))]
pub struct KeygenPartyId(pub u16);

/// A Offline Party Id, in the range [1, t+1], where t is the signing threshold.
///
/// This is a wrapper around u16 to prevent the misuse of the party id as a keygen party id, for
/// example.
///
/// To construct a OfflinePartyId, use the [`Self::try_from_keygen_party_id`] method.
#[derive(
Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, codec::Encode, codec::Decode,
)]
#[cfg_attr(feature = "scale-info", derive(scale_info::TypeInfo))]
pub struct OfflinePartyId(u16);

impl TryFrom<u16> for KeygenPartyId {
    type Error = crate::error::Error;
    /// This the only where you can construct a KeygenPartyId
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        // party_i starts from 1
        if value == 0 {
            Err(crate::error::Error::InvalidKeygenPartyId)
        } else {
            Ok(Self(value))
        }
    }
}

impl AsRef<u16> for KeygenPartyId {
    fn as_ref(&self) -> &u16 {
        &self.0
    }
}

impl fmt::Display for KeygenPartyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl KeygenPartyId {
    /// Try to convert a KeygenPartyId to an OfflinePartyId.
    pub fn try_to_offline_party_id(&self, s_l: &[Self]) -> Result<OfflinePartyId, crate::error::Error> {
        OfflinePartyId::try_from_keygen_party_id(*self, s_l)
    }

    /// Converts the PartyId to an index in the range [0, n-1].
    ///
    /// The implementation is safe because the party id is guaranteed to be in the range [1, n].
    pub const fn to_index(&self) -> usize {
        self.0 as usize - 1
    }
}

impl AsRef<u16> for OfflinePartyId {
    fn as_ref(&self) -> &u16 {
        &self.0
    }
}

impl fmt::Display for OfflinePartyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl OfflinePartyId {
    /// Creates a OfflinePartyId from a KeygenPartyId and a list of the signing parties.
    ///
    /// This finds the index of the KeygenPartyId in the list of signing parties, then we use that
    /// index as OfflinePartyId.
    ///
    /// This is safe because the KeygenPartyId is guaranteed to be in the range `[1, n]`, and the
    /// OfflinePartyId is guaranteed to be in the range `[1, t+1]`. if the KeygenPartyId is not in
    /// the list of signing parties, then we return an error.
    pub fn try_from_keygen_party_id(
        i: KeygenPartyId,
        s_l: &[KeygenPartyId],
    ) -> Result<Self, crate::error::Error> {
        // find the index of the party in the list of signing parties
        let index = s_l.iter().position(|&x| x == i).ok_or(crate::error::Error::InvalidKeygenPartyId)?;
        let offline_id = index as u16 + 1;
        Ok(Self(offline_id))
    }

    /// Tries to Converts the `OfflinePartyId` to a `KeygenPartyId`.
    ///
    /// Returns an error if the `OfflinePartyId` is not in the list of signing parties.
    pub fn try_to_keygen_party_id(&self, s_l: &[KeygenPartyId]) -> Result<KeygenPartyId, crate::error::Error> {
        let idx = self.to_index();
        let party_i = s_l.get(idx).cloned().ok_or(crate::error::Error::InvalidSigningSet)?;
        Ok(party_i)
    }

    /// Converts the OfflinePartyId to an index.
    pub const fn to_index(&self) -> usize {
        self.0 as usize - 1
    }
}