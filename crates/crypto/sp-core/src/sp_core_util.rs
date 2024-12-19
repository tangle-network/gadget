use gadget_std::{
    string::{String, ToString},
    vec::Vec,
};
use sp_core::ByteArray;
use sp_core::Pair;

/// Implements serde and KeyType trait for any sp_core crypto type.
///
/// Implements common functionality for key pairs and public keys.
macro_rules! impl_sp_core_pair_public {
    ($key_type:ident, $pair_type:ty, $public:ty) => {
        paste::paste! {
            /// Wrapper struct for the cryptographic key pair.
            ///
            /// This provides a safe interface for serialization and deserialization
            /// of the underlying key pair type.
            #[derive(Clone)]
            pub struct [<$key_type Pair>](pub $pair_type);

            impl PartialEq for [<$key_type Pair>] {
                fn eq(&self, other: &Self) -> bool {
                    self.0.to_raw_vec() == other.0.to_raw_vec()
                }
            }

            impl Eq for [<$key_type Pair>] {}

            impl PartialOrd for [<$key_type Pair>] {
                fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for [<$key_type Pair>] {
                fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                    self.0.to_raw_vec().cmp(&other.0.to_raw_vec())
                }
            }

            impl gadget_std::fmt::Debug for [<$key_type Pair>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.0.to_raw_vec())
                }
            }

            impl serde::Serialize for [<$key_type Pair>] {
                fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    let bytes = self.0.to_raw_vec();
                    Vec::serialize(&bytes, serializer)
                }
            }

            impl<'de> serde::Deserialize<'de> for [<$key_type Pair>] {
                fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    let bytes = <Vec<u8>>::deserialize(deserializer)?;
                    let pair = <$pair_type>::from_seed_slice(&bytes)
                        .map_err($crate::error::SecretStringErrorWrapper)
                        .map_err(|e| serde::de::Error::custom(e.to_string()))?;
                    Ok([<$key_type Pair>](pair))
                }
            }

            /// Wrapper struct for the cryptographic public key.
            #[derive(Clone, serde::Serialize, serde::Deserialize)]
            pub struct [<$key_type Public>](pub <$pair_type as sp_core::Pair>::Public);

            impl PartialEq for [<$key_type Public>]{
                fn eq(&self, other: &Self) -> bool {
                    self.0 == other.0
                }
            }

            impl Eq for [<$key_type Public>]{}

            impl PartialOrd for [<$key_type Public>]{
                fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for [<$key_type Public>]{
                fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                    self.0.to_raw_vec().cmp(&other.0.to_raw_vec())
                }
            }

            impl gadget_std::fmt::Debug for [<$key_type Public>]{
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.0.to_raw_vec())
                }
            }
        }
    };
}

/// Implements signature functionality for non-BLS signatures.
macro_rules! impl_sp_core_signature {
    ($key_type:ident, $pair_type:ty) => {
        paste::paste! {
            #[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
            pub struct [<$key_type Signature>](pub <$pair_type as sp_core::Pair>::Signature);

            impl PartialOrd for [<$key_type Signature>] {
                fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for [<$key_type Signature>] {
                fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                    self.0.0.cmp(&other.0.0)
                }
            }

            impl gadget_std::fmt::Debug for [<$key_type Signature>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.0.0)
                }
            }
        }
    };
}

/// Implements KeyType trait for non-BLS signatures.
macro_rules! impl_sp_core_key_type {
    ($key_type:ident, $pair_type:ty) => {
        paste::paste! {
            pub struct $key_type;

            impl gadget_crypto_core::KeyType for $key_type {
                type Public = [<$key_type Public>];
                type Secret = [<$key_type Pair>];
                type Signature = [<$key_type Signature>];
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
                            Ok([<$key_type Pair>](pair))
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
                            Ok([<$key_type Pair>](pair))
                        }
                    }
                }

                fn generate_with_string(secret: String) -> $crate::error::Result<Self::Secret> {
                    let pair = <$pair_type>::from_string(&secret, None)
                        .map_err(|_| $crate::error::SpCoreError::InvalidSeed("Invalid secret string".to_string()))?;
                    Ok([<$key_type Pair>](pair))
                }

                fn public_from_secret(secret: &Self::Secret) -> Self::Public {
                    [<$key_type Public>](secret.0.public())
                }

                fn sign_with_secret(
                    secret: &mut Self::Secret,
                    msg: &[u8],
                ) -> $crate::error::Result<Self::Signature> {
                    Ok([<$key_type Signature>](secret.0.sign(msg)))
                }

                fn sign_with_secret_pre_hashed(
                    secret: &mut Self::Secret,
                    msg: &[u8; 32],
                ) -> $crate::error::Result<Self::Signature> {
                    Ok([<$key_type Signature>](secret.0.sign(msg)))
                }

                fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
                    <$pair_type as sp_core::Pair>::verify(&signature.0, msg, &public.0)
                }
            }

            impl [<$key_type Pair>] {
                pub fn public(&self) -> [<$key_type Public>] {
                    [<$key_type Public>](self.0.public())
                }
            }

            impl gadget_std::ops::Deref for [<$key_type Pair>] {
                type Target = $pair_type;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl gadget_std::ops::DerefMut for [<$key_type Pair>] {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }
        }
    };
}

/// Implements both pair/public and signature traits for a given sp_core crypto type
macro_rules! impl_sp_core_crypto {
    ($key_type:ident, $module:ident) => {
        impl_sp_core_pair_public!($key_type, sp_core::$module::Pair, sp_core::$module::Public);
        impl_sp_core_signature!($key_type, sp_core::$module::Pair);
        impl_sp_core_key_type!($key_type, sp_core::$module::Pair);
    };
}

impl_sp_core_crypto!(SpEcdsa, ecdsa);
impl_sp_core_crypto!(SpEd25519, ed25519);
impl_sp_core_crypto!(SpSr25519, sr25519);

impl Copy for SpEcdsaPublic {}
impl Copy for SpEd25519Public {}
impl Copy for SpSr25519Public {}
