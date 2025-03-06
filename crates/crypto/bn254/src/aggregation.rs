use crate::{error::Bn254Error, ArkBlsBn254, ArkBlsBn254Public, ArkBlsBn254Signature};
use ark_bn254::{G1Affine, G2Affine};
use ark_ec::AffineRepr;
use ark_ff::AdditiveGroup;
use gadget_crypto_core::{aggregation::AggregatableSignature, KeyType};

impl AggregatableSignature for ArkBlsBn254 {
    fn verify_single(
        message: &[u8],
        signature: &ArkBlsBn254Signature,
        public_key: &ArkBlsBn254Public,
    ) -> bool {
        ArkBlsBn254::verify(public_key, message, signature)
    }

    fn verify_aggregate(
        message: &[u8],
        signature: &ArkBlsBn254Signature,
        public_keys: &[ArkBlsBn254Public],
    ) -> bool {
        let mut aggregated_public_key: G2Affine = G2Affine::zero();

        for key in public_keys {
            let value = aggregated_public_key + key.0;
            aggregated_public_key = value.into();
        }

        ArkBlsBn254::verify(
            &ArkBlsBn254Public(aggregated_public_key),
            message,
            signature,
        )
    }

    fn aggregate(signatures: &[ArkBlsBn254Signature]) -> Result<ArkBlsBn254Signature, Bn254Error> {
        let mut aggregated_signature = G1Affine::zero();
        for signature in signatures.iter() {
            let value = aggregated_signature + signature.0;
            aggregated_signature = value.into();
        }

        Ok(ArkBlsBn254Signature(aggregated_signature))
    }
}
