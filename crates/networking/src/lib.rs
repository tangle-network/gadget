#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod behaviours;
pub mod blueprint_protocol;
pub mod discovery;
pub mod error;
pub mod handlers;
pub mod service;
pub mod service_handle;
pub mod types;

#[cfg(feature = "round-based-compat")]
pub use gadget_networking_round_based_extension as round_based_compat;

#[cfg(test)]
mod tests;

pub use key_types::*;
pub use service::{NetworkConfig, NetworkEvent, NetworkService};

#[cfg(all(
    feature = "sp-core-ecdsa",
    not(feature = "sp-core-sr25519"),
    not(feature = "sp-core-ed25519")
))]
pub mod key_types {
    pub use gadget_crypto::sp_core::{
        SpEcdsa as Curve, SpEcdsaPair as InstanceMsgKeyPair, SpEcdsaPublic as InstanceMsgPublicKey,
        SpEcdsaSignature as InstanceSignedMsgSignature,
    };

    impl super::KeySignExt for InstanceMsgKeyPair {
        fn sign_prehash(&self, prehash: &[u8; 32]) -> InstanceSignedMsgSignature {
            InstanceSignedMsgSignature(self.0.sign_prehashed(prehash))
        }
    }
}

#[cfg(all(
    feature = "sp-core-sr25519",
    not(feature = "sp-core-ecdsa"),
    not(feature = "sp-core-ed25519")
))]
pub mod key_types {
    pub use gadget_crypto::sp_core::{
        SpSr25519 as Curve, SpSr25519Pair as InstanceMsgKeyPair,
        SpSr25519Public as InstanceMsgPublicKey, SpSr25519Signature as InstanceSignedMsgSignature,
    };

    impl super::KeySignExt for InstanceMsgKeyPair {
        fn sign_prehash(&self, prehash: &[u8; 32]) -> InstanceSignedMsgSignature {
            InstanceSignedMsgSignature(self.0.sign_prehashed(prehash))
        }
    }
}

#[cfg(all(
    feature = "sp-core-ed25519",
    not(feature = "sp-core-ecdsa"),
    not(feature = "sp-core-sr25519")
))]
pub mod key_types {
    pub use gadget_crypto::sp_core::{
        SpEd25519 as Curve, SpEd25519Pair as InstanceMsgKeyPair,
        SpEd25519Public as InstanceMsgPublicKey, SpEd25519Signature as InstanceSignedMsgSignature,
    };

    impl super::KeySignExt for InstanceMsgKeyPair {
        fn sign_prehash(&self, prehash: &[u8; 32]) -> InstanceSignedMsgSignature {
            InstanceSignedMsgSignature(self.0.sign_prehashed(prehash))
        }
    }
}

#[cfg(all(
    not(feature = "sp-core-ecdsa"),
    not(feature = "sp-core-sr25519"),
    not(feature = "sp-core-ed25519")
))]
pub mod key_types {
    // Default to k256 ECDSA implementation
    pub use gadget_crypto::k256::{
        K256Ecdsa as Curve, K256Signature as InstanceSignedMsgSignature,
        K256SigningKey as InstanceMsgKeyPair, K256VerifyingKey as InstanceMsgPublicKey,
    };

    impl super::KeySignExt for InstanceMsgKeyPair {
        fn sign_prehash(&self, prehash: &[u8; 32]) -> InstanceSignedMsgSignature {
            self.sign_prehash(prehash)
        }
    }
}

// Compile-time assertion to ensure only one feature is enabled
#[cfg(any(
    all(feature = "sp-core-ecdsa", feature = "sp-core-sr25519"),
    all(feature = "sp-core-ecdsa", feature = "sp-core-ed25519"),
    all(feature = "sp-core-sr25519", feature = "sp-core-ed25519")
))]
compile_error!(
    "Only one of 'sp-core-ecdsa', 'sp-core-sr25519', or 'sp-core-ed25519' features can be enabled at a time"
);

pub(crate) trait KeySignExt {
    fn sign_prehash(&self, prehash: &[u8; 32]) -> InstanceSignedMsgSignature;
}
