#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod behaviours;
pub mod blueprint_protocol;
pub mod discovery;
pub mod error;
pub mod handlers;
pub mod service;
pub mod service_handle;
pub mod types;

#[cfg(test)]
mod tests;

pub use gadget_crypto::KeyType;
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
}

#[cfg(all(
    feature = "bls381",
    not(feature = "sp-core-ecdsa"),
    not(feature = "sp-core-sr25519"),
    not(feature = "sp-core-ed25519")
))]
pub mod key_types {
    // Default to k256 ECDSA implementation
    pub use gadget_crypto::bls::bls381::{
        W3fBls381 as Curve, W3fBls381Public as InstanceMsgPublicKey,
        W3fBls381Secret as InstanceMsgKeyPair, W3fBls381Signature as InstanceSignedMsgSignature,
    };
}

#[cfg(all(
    not(feature = "sp-core-ecdsa"),
    not(feature = "sp-core-sr25519"),
    not(feature = "sp-core-ed25519"),
    not(feature = "bls381")
))]
pub mod key_types {
    // Default to k256 ECDSA implementation
    pub use gadget_crypto::k256::{
        K256Ecdsa as Curve, K256Signature as InstanceSignedMsgSignature,
        K256SigningKey as InstanceMsgKeyPair, K256VerifyingKey as InstanceMsgPublicKey,
    };
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
