#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;

use gadget_std::vec::Vec;
pub use sp_core;
use sp_core::DeriveJunction;
use sp_core::crypto::DeriveError;
use sp_core::crypto::SecretStringError;
use subxt_core::config::PolkadotConfig;
use subxt_core::tx::signer::Signer;
use tangle_subxt::subxt_core;
use tangle_subxt::subxt_core::Config;

#[derive(Clone, Debug)]
pub struct TanglePairSigner<Pair> {
    pub(crate) pair: pair_signer::PairSigner<Pair>,
}

impl<Pair: sp_core::Pair> sp_core::crypto::CryptoType for TanglePairSigner<Pair> {
    type Pair = Pair;
}

impl<Pair: sp_core::Pair> TanglePairSigner<Pair>
where
    <Pair as sp_core::Pair>::Signature: pair_signer::IntoMultiSignature,
    sp_runtime::MultiSigner: From<<Pair as sp_core::Pair>::Public>,
{
    pub fn new(pair: Pair) -> Self {
        TanglePairSigner {
            pair: pair_signer::PairSigner::new(pair),
        }
    }

    pub fn into_inner(self) -> pair_signer::PairSigner<Pair> {
        self.pair
    }

    pub fn signer(&self) -> &Pair {
        self.pair.signer()
    }
}

impl<Pair> Signer<PolkadotConfig> for TanglePairSigner<Pair>
where
    Pair: sp_core::Pair,
    <Pair as sp_core::Pair>::Signature: pair_signer::IntoMultiSignature,
    sp_runtime::MultiSigner: From<<Pair as sp_core::Pair>::Public>,
{
    fn account_id(&self) -> <PolkadotConfig as Config>::AccountId {
        self.pair.account_id().clone()
    }

    fn address(&self) -> <PolkadotConfig as Config>::Address {
        self.pair.address()
    }

    fn sign(&self, signer_payload: &[u8]) -> <PolkadotConfig as Config>::Signature {
        self.pair.sign(signer_payload)
    }
}

impl<Pair: sp_core::Pair> sp_core::Pair for TanglePairSigner<Pair>
where
    <Pair as sp_core::Pair>::Signature: pair_signer::IntoMultiSignature,
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
                    pair: pair_signer::PairSigner::new(pair),
                },
                seed,
            )
        })
    }

    fn from_seed_slice(seed: &[u8]) -> Result<Self, SecretStringError> {
        Pair::from_seed_slice(seed).map(|pair| TanglePairSigner {
            pair: pair_signer::PairSigner::new(pair),
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

pub mod pair_signer {
    use super::Signer;
    use crate::Config;
    use sp_runtime::traits::IdentifyAccount;
    use tangle_subxt::subxt_core::config::PolkadotConfig;
    use tangle_subxt::subxt_core::utils::MultiSignature;

    pub trait IntoMultiSignature {
        fn into(self) -> MultiSignature;
    }

    impl IntoMultiSignature for sp_core::sr25519::Signature {
        fn into(self) -> MultiSignature {
            MultiSignature::Sr25519(self.0)
        }
    }

    impl IntoMultiSignature for sp_core::ed25519::Signature {
        fn into(self) -> MultiSignature {
            MultiSignature::Ed25519(self.0)
        }
    }

    impl IntoMultiSignature for sp_core::ecdsa::Signature {
        fn into(self) -> MultiSignature {
            MultiSignature::Ecdsa(self.0)
        }
    }

    /// A [`Signer`] implementation that can be constructed from an [`sp_core::Pair`].
    #[derive(Clone, Debug)]
    pub struct PairSigner<Pair> {
        account_id: <PolkadotConfig as Config>::AccountId,
        signer: Pair,
    }

    impl<Pair> PairSigner<Pair>
    where
        Pair: sp_core::Pair,
        <Pair as sp_core::Pair>::Signature: IntoMultiSignature,
        sp_runtime::MultiSigner: From<<Pair as sp_core::Pair>::Public>,
    {
        /// Creates a new [`Signer`] from an [`sp_core::Pair`].
        pub fn new(signer: Pair) -> Self {
            let account_id = sp_runtime::MultiSigner::from(signer.public());
            Self {
                account_id: <PolkadotConfig as Config>::AccountId::from(
                    *AsRef::<[u8; 32]>::as_ref(&account_id.into_account()),
                ),
                signer,
            }
        }

        /// Returns the [`sp_core::Pair`] implementation used to construct this.
        pub fn signer(&self) -> &Pair {
            &self.signer
        }

        /// Return the account ID.
        pub fn account_id(&self) -> &<PolkadotConfig as Config>::AccountId {
            &self.account_id
        }
    }

    impl<Pair> Signer<PolkadotConfig> for PairSigner<Pair>
    where
        Pair: sp_core::Pair,
        <Pair as sp_core::Pair>::Signature: IntoMultiSignature,
        sp_runtime::MultiSigner: From<<Pair as sp_core::Pair>::Public>,
    {
        fn account_id(&self) -> <PolkadotConfig as Config>::AccountId {
            self.account_id.clone()
        }

        fn address(&self) -> <PolkadotConfig as Config>::Address {
            self.account_id.clone().into()
        }

        fn sign(&self, signer_payload: &[u8]) -> <PolkadotConfig as Config>::Signature {
            IntoMultiSignature::into(self.signer.sign(signer_payload))
        }
    }
}
