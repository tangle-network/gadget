#[cfg(test)]
mod tests;

use gadget_crypto_core::KeyEncoding;
use gadget_std::{
    string::{String, ToString},
    vec::Vec,
};
use sp_core::Pair;

/// Implements serde and KeyType trait for any sp_core crypto type.
///
/// Implements common functionality for key pairs and public keys.
macro_rules! impl_sp_core_bls_pair_public {
    ($key_type:ident, $pair_type:ty, $public:ty) => {
        paste::paste! {
            /// Wrapper struct for the cryptographic key pair.
            ///
            /// This provides a safe interface for serialization and deserialization
            /// of the underlying key pair type.
            #[derive(Clone)]
            pub struct [<Sp $key_type Pair>](pub $pair_type);

            impl PartialEq for [<Sp $key_type Pair>] {
                fn eq(&self, other: &Self) -> bool {
                    self.to_bytes() == other.to_bytes()
                }
            }

            impl Eq for [<Sp $key_type Pair>] {}

            impl PartialOrd for [<Sp $key_type Pair>] {
                fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for [<Sp $key_type Pair>] {
                fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                    self.to_bytes().cmp(&other.to_bytes())
                }
            }

            impl gadget_std::fmt::Debug for [<Sp $key_type Pair>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.to_bytes())
                }
            }

            impl KeyEncoding for [<Sp $key_type Pair>] {
                fn to_bytes(&self) -> Vec<u8> {
                    self.0.to_raw_vec()
                }

                fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, serde::de::value::Error> {
                    <$pair_type>::from_seed_slice(&bytes)
                        .map_err(|e| serde::de::Error::custom(format!("Failed to create pair from seed: {}", e)))
                        .map(Self)
                }
            }

            impl serde::Serialize for [<Sp $key_type Pair>] {
                fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    // Get the raw seed bytes
                    let bytes = self.to_bytes();
                    // Serialize as raw bytes
                    <Vec::<u8>>::serialize(&bytes, serializer)
                }
            }

            impl<'de> serde::Deserialize<'de> for [<Sp $key_type Pair>] {
                fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    // Deserialize directly into a Vec<u8> for the seed
                    let seed = <Vec::<u8>>::deserialize(deserializer)
                        .map_err(|e| serde::de::Error::custom(format!("Failed to deserialize seed bytes: {}", e)))?;

                    // Generate a new pair from the seed bytes
                    Self::from_bytes(seed.as_slice()).map_err(|e| serde::de::Error::custom(e.to_string()))
                }
            }

            /// Wrapper struct for the cryptographic public key.
            #[derive(Clone, serde::Serialize, serde::Deserialize)]
            pub struct [<Sp $key_type Public>](pub <$pair_type as sp_core::Pair>::Public);

            impl PartialEq for [<Sp $key_type Public>]{
                fn eq(&self, other: &Self) -> bool {
                    self.0 == other.0
                }
            }

            impl Eq for [<Sp $key_type Public>]{}

            impl PartialOrd for [<Sp $key_type Public>]{
                fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for [<Sp $key_type Public>]{
                fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                    self.to_bytes().cmp(&other.to_bytes())
                }
            }

            impl gadget_std::fmt::Debug for [<Sp $key_type Public>]{
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.to_bytes())
                }
            }

            impl KeyEncoding for [<Sp $key_type Public>] {
                fn to_bytes(&self) -> Vec<u8> {
                    self.0.to_raw().to_vec()
                }

                fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, serde::de::value::Error> {
                    let bytes = bytes.try_into().map_err(|_| serde::de::Error::custom("Invalid public key length"))?;
                    Ok(Self(<$pair_type as sp_core::Pair>::Public::from_raw(bytes)))
                }
            }
        }
    };
}

/// Implements KeyType trait for non-BLS signatures.
macro_rules! impl_sp_core_bls_key_type {
    ($key_type:ident, $pair_type:ty) => {
        paste::paste! {
            pub struct [<Sp $key_type>];

            impl gadget_crypto_core::KeyType for [<Sp $key_type>] {
                type Public = [<Sp $key_type Public>];
                type Secret = [<Sp $key_type Pair>];
                type Signature = [<Sp $key_type Signature>];
                type Error = $crate::error::SpCoreError;

                fn key_type_id() -> gadget_crypto_core::KeyTypeId {
                    gadget_crypto_core::KeyTypeId::$key_type
                }

                fn generate_with_seed(seed: Option<&[u8]>) -> $crate::error::Result<Self::Secret> {
                    match seed {
                        Some(seed) => {
                            let pair = <$pair_type>::from_seed_slice(seed)
                                .map_err($crate::error::SecretStringErrorWrapper)
                                .map_err(Into::<$crate::error::SpCoreError>::into)?;
                            Ok([<Sp $key_type Pair>](pair))
                        }
                        None => {
                            #[cfg(feature = "std")]
                            let (pair, _) = <$pair_type>::generate();
                            #[cfg(not(feature = "std"))]
                            let pair = {
                                use gadget_std::Rng;
                                let mut seed = Self::get_test_rng().gen::<[u8; 32]>();
                                <$pair_type>::from_seed_slice(&mut seed)
                                    .map_err($crate::error::SecretStringErrorWrapper)
                                    .map_err(Into::<$crate::error::SpCoreError>::into)?
                            };
                            Ok([<Sp $key_type Pair>](pair))
                        }
                    }
                }

                fn generate_with_string(secret: String) -> $crate::error::Result<Self::Secret> {
                    let pair = <$pair_type>::from_string(&secret, None)
                        .map_err(|_| $crate::error::SpCoreError::InvalidSeed("Invalid secret string".to_string()))?;
                    Ok([<Sp $key_type Pair>](pair))
                }

                fn public_from_secret(secret: &Self::Secret) -> Self::Public {
                    [<Sp $key_type Public>](secret.0.public())
                }

                fn sign_with_secret(
                    secret: &mut Self::Secret,
                    msg: &[u8],
                ) -> $crate::error::Result<Self::Signature> {
                    Ok([<Sp $key_type Signature>](secret.0.sign(msg)))
                }

                fn sign_with_secret_pre_hashed(
                    secret: &mut Self::Secret,
                    msg: &[u8; 32],
                ) -> $crate::error::Result<Self::Signature> {
                    Ok([<Sp $key_type Signature>](secret.0.sign(msg)))
                }

                fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
                    <$pair_type as sp_core::Pair>::verify(&signature.0, msg, &public.0)
                }
            }

            impl [<Sp $key_type Pair>] {
                pub fn public(&self) -> [<Sp $key_type Public>] {
                    [<Sp $key_type Public>](self.0.public())
                }
            }

            impl gadget_std::ops::Deref for [<Sp $key_type Pair>] {
                type Target = $pair_type;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl gadget_std::ops::DerefMut for [<Sp $key_type Pair>] {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }
        }
    };
}

/// Implements signature functionality for BLS signatures.
macro_rules! impl_sp_core_bls_signature {
    ($key_type:ident, $signature:ty) => {
        paste::paste! {
            #[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
            pub struct [<Sp $key_type Signature>](pub $signature);

            impl PartialOrd for [<Sp $key_type Signature>] {
                fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for [<Sp $key_type Signature>] {
                fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                    let self_bytes: &[u8] = self.0.as_ref();
                    let other_bytes: &[u8] = other.0.as_ref();
                    self_bytes.cmp(other_bytes)
                }
            }

            impl gadget_std::fmt::Debug for [<Sp $key_type Signature>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    let bytes: &[u8] = self.0.as_ref();
                    write!(f, "{:?}", bytes)
                }
            }
        }
    };
}

/// Implements both pair/public and signature traits for a given sp_core crypto type
macro_rules! impl_sp_core_bls_crypto {
    ($key_type:ident, $module:ident) => {
        impl_sp_core_bls_pair_public!($key_type, sp_core::$module::Pair, sp_core::$module::Public);
        impl_sp_core_bls_signature!($key_type, sp_core::$module::Signature);
        impl_sp_core_bls_key_type!($key_type, sp_core::$module::Pair);
    };
}

impl_sp_core_bls_crypto!(Bls377, bls377);
impl_sp_core_bls_crypto!(Bls381, bls381);
