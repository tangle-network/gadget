//! Ed25519 Support using [`ed25519-zebra`] in pure Rust.

pub use ed25519_zebra::SigningKey as Secret;
pub use ed25519_zebra::VerificationKey as Public;
pub use ed25519_zebra::Signature;
