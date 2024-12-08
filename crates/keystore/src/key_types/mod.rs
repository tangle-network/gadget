//! This module contains the key types that are supported by the keystore.

/// Tangle specific key types
#[cfg(feature = "tangle")]
pub mod sp_bls377;
#[cfg(feature = "tangle")]
pub mod sp_bls381;
#[cfg(feature = "tangle")]
pub mod sp_ecdsa;
#[cfg(feature = "tangle")]
pub mod sp_ed25519;
#[cfg(feature = "tangle")]
pub mod sp_sr25519;

/// Key types that are used separately from Tangle defaults
#[cfg(feature = "bn254")]
pub mod ark_bn254;
#[cfg(feature = "ecdsa")]
pub mod k256_ecdsa;
#[cfg(feature = "sr25519-schnorrkel")]
pub mod schnorrkel_sr25519;
#[cfg(feature = "bls381")]
pub mod w3f_bls381;
#[cfg(feature = "zebra")]
pub mod zebra_ed25519;

// Re-export the types
#[cfg(feature = "bn254")]
pub use self::ark_bn254::*;
#[cfg(feature = "ecdsa")]
pub use self::k256_ecdsa::*;
#[cfg(feature = "bls381")]
pub use self::w3f_bls381::*;
#[cfg(feature = "zebra")]
pub use self::zebra_ed25519::*;

// Re-export the types
#[cfg(feature = "tangle")]
pub use self::sp_bls377::*;
#[cfg(feature = "tangle")]
pub use self::sp_bls381::*;
#[cfg(feature = "tangle")]
pub use self::sp_ecdsa::*;
#[cfg(feature = "tangle")]
pub use self::sp_ed25519::*;
#[cfg(feature = "tangle")]
pub use self::sp_sr25519::*;
