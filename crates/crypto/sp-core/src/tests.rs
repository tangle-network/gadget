use super::*;

mod ecdsa_crypto_tests {
    use super::*;
    gadget_crypto_core::impl_crypto_tests!(SpEcdsa, SpEcdsaPair, SpEcdsaSignature);
}

mod ed25519_crypto_tests {
    use super::*;
    gadget_crypto_core::impl_crypto_tests!(SpEd25519, SpEd25519Pair, SpEd25519Signature);
}

mod sr25519_crypto_tests {
    use super::*;
    gadget_crypto_core::impl_crypto_tests!(SpSr25519, SpSr25519Pair, SpSr25519Signature);
}
