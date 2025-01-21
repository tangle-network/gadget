#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;

use gadget_std::vec::Vec;
use sp_core::crypto::DeriveError;
use sp_core::crypto::SecretStringError;
use sp_core::DeriveJunction;
use subxt_core::config::PolkadotConfig;
pub use subxt_core::ext::sp_core;
use subxt_core::tx::signer::{PairSigner, Signer};
use subxt_core::utils::{AccountId32, MultiAddress, MultiSignature};
use tangle_subxt::subxt_core;

#[derive(Clone, Debug)]
pub struct TanglePairSigner<Pair> {
    pub(crate) pair: PairSigner<PolkadotConfig, Pair>,
}

impl<Pair: sp_core::Pair> sp_core::crypto::CryptoType for TanglePairSigner<Pair> {
    type Pair = Pair;
}

impl<Pair: sp_core::Pair> TanglePairSigner<Pair>
where
    <Pair as sp_core::Pair>::Signature: Into<MultiSignature>,
    sp_runtime::MultiSigner: From<<Pair as sp_core::Pair>::Public>,
{
    pub fn new(pair: Pair) -> Self {
        TanglePairSigner {
            pair: PairSigner::new(pair),
        }
    }

    pub fn into_inner(self) -> PairSigner<PolkadotConfig, Pair> {
        self.pair
    }

    pub fn signer(&self) -> &Pair {
        self.pair.signer()
    }
}

impl<Pair> Signer<PolkadotConfig> for TanglePairSigner<Pair>
where
    Pair: sp_core::Pair,
    Pair::Signature: Into<MultiSignature>,
{
    fn account_id(&self) -> AccountId32 {
        self.pair.account_id()
    }

    fn address(&self) -> MultiAddress<AccountId32, ()> {
        self.pair.address()
    }

    fn sign(&self, signer_payload: &[u8]) -> MultiSignature {
        self.pair.sign(signer_payload)
    }
}

impl<Pair: sp_core::Pair> sp_core::Pair for TanglePairSigner<Pair>
where
    <Pair as sp_core::Pair>::Signature: Into<MultiSignature>,
    sp_runtime::MultiSigner: From<<Pair as sp_core::Pair>::Public>,
{
    type Public = Pair::Public;
    type Seed = Pair::Seed;
    type Signature = Pair::Signature;

    fn derive<Iter: Iterator<Item = DeriveJunction>>(
        &self,
        path: Iter,
        seed: Option<Self::Seed>,
    ) -> Result<(Self, Option<Self::Seed>), DeriveError> {
        Pair::derive(self.pair.signer(), path, seed).map(|(pair, seed)| {
            (
                TanglePairSigner {
                    pair: PairSigner::new(pair),
                },
                seed,
            )
        })
    }

    fn from_seed_slice(seed: &[u8]) -> Result<Self, SecretStringError> {
        Pair::from_seed_slice(seed).map(|pair| TanglePairSigner {
            pair: PairSigner::new(pair),
        })
    }

    fn sign(&self, message: &[u8]) -> Self::Signature {
        Pair::sign(self.pair.signer(), message)
    }

    fn verify<M: AsRef<[u8]>>(sig: &Self::Signature, message: M, pubkey: &Self::Public) -> bool {
        Pair::verify(sig, message, pubkey)
    }

    fn public(&self) -> Self::Public {
        Pair::public(self.pair.signer())
    }

    fn to_raw_vec(&self) -> Vec<u8> {
        Pair::to_raw_vec(self.pair.signer())
    }
}

#[cfg(feature = "evm")]
impl TanglePairSigner<sp_core::ecdsa::Pair> {
    /// Returns the alloy-compatible key for the ECDSA key pair.
    pub fn alloy_key(
        &self,
    ) -> error::Result<alloy_signer_local::LocalSigner<k256::ecdsa::SigningKey>> {
        let k256_ecdsa_secret_key = self.pair.signer().seed();
        let res = alloy_signer_local::LocalSigner::from_slice(&k256_ecdsa_secret_key)?;
        Ok(res)
    }

    /// Returns the Alloy Address for the ECDSA key pair.
    pub fn alloy_address(&self) -> error::Result<alloy_primitives::Address> {
        Ok(self.alloy_key()?.address())
    }
}
