use crate::backend::{Error, KeyType};
use alloy_primitives::keccak256;
use ark_bn254::{Fr, G1Affine, G2Affine};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{BigInt, PrimeField, UniformRand};
use rust_bls_bn254::{sign, verify};
use std::{fmt::format, str::FromStr};

/// BLS-BN254 key type
pub struct ArkBlsBn254;

impl KeyType for ArkBlsBn254 {
    type Public = G2Affine;
    type Secret = Fr;
    type Signature = G1Affine;

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret, Error> {
        let secret = if let Some(seed) = seed {
            let seed = std::str::from_utf8(seed).map_err(|e| Error::InvalidSeed(e.to_string()))?;
            Fr::from_str(seed).map_err(|e| Error::InvalidSeed(format!("{:?}", e)))?
        } else {
            // Should only be used for testing. Pass a seed in production.
            let mut rng = gadget_std::test_rng();
            Fr::rand(&mut rng)
        };
        Ok(secret)
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        G2Affine::generator()
            .mul_bigint(secret.into_bigint())
            .into_affine()
    }

    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature, Error> {
        // Bn254 signing hashes to curve in the signature function
        let signature = sign(*secret, &msg).map_err(|e| Error::SignatureFailed(e.to_string()))?;
        Ok(signature)
    }

    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature, Error> {
        let signature = sign(*secret, msg).map_err(|e| Error::SignatureFailed(e.to_string()))?;
        Ok(signature)
    }

    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
        verify(*public, msg, *signature)
    }
}
