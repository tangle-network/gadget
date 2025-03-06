use crate::{
    bls377::{W3fBls377, W3fBls377Public, W3fBls377Signature},
    bls381::{W3fBls381, W3fBls381Public, W3fBls381Signature},
    error::BlsError,
};
use gadget_crypto_core::{aggregation::AggregatableSignature, BytesEncoding, KeyType};
use gadget_std::Zero;
use tnt_bls::{single_pop_aggregator::SignatureAggregatorAssumingPoP, *};

macro_rules! impl_aggregatable_signature {
    ($key_type:ident, $tiny_engine:ty) => {
        paste::paste! {
            impl AggregatableSignature for [<W3f $key_type>] {
                fn verify_single(
                    message: &[u8],
                    signature: &[<W3f $key_type Signature>],
                    public_key: &[<W3f $key_type Public>],
                ) -> bool {
                    [<W3f $key_type>]::verify(public_key, message, signature)
                }

                fn verify_aggregate(
                    message: &[u8],
                    signature: &[<W3f $key_type Signature>],
                    public_keys: &[[<W3f $key_type Public>]],
                ) -> bool {
                    let sig = match tnt_bls::Signature::<$tiny_engine>::from_bytes(&signature.to_bytes()[..]) {
                        Ok(s) => s,
                        Err(_) => return false,
                    };
                    let mut aggregated_public_key =
                        PublicKey::<$tiny_engine>(<$tiny_engine as EngineBLS>::PublicKeyGroup::zero());

                    let mut double_pubkeys: Vec<DoublePublicKey<$tiny_engine>> = vec![];

                    for key in public_keys {
                        let public_key = match tnt_bls::double::DoublePublicKey::<$tiny_engine>::from_bytes(
                            &key.to_bytes()[..],
                        ) {
                            Ok(pk) => pk,
                            Err(_) => return false,
                        };
                        aggregated_public_key.0 += public_key.1.clone();
                        double_pubkeys.push(public_key);
                    }

                    let public_keys_in_sig_grp: Vec<PublicKeyInSignatureGroup<$tiny_engine>> = double_pubkeys
                        .iter()
                        .map(|k| PublicKeyInSignatureGroup(k.0))
                        .collect();

                    let message: Message = Message::new(b"", message);
                    let mut prover_aggregator =
                        SignatureAggregatorAssumingPoP::<$tiny_engine>::new(message.clone());
                    prover_aggregator.add_signature(&sig);
                    let mut verifier_aggregator = SignatureAggregatorAssumingPoP::<$tiny_engine>::new(message);
                    //get the signature and already aggregated public key from the prover
                    verifier_aggregator.add_signature(&(&prover_aggregator).signature());
                    verifier_aggregator.add_publickey(&aggregated_public_key);

                    // aggregate public keys in signature group
                    public_keys_in_sig_grp.iter().for_each(|pk| {
                        verifier_aggregator.add_auxiliary_public_key(pk);
                    });

                    verifier_aggregator.verify_using_aggregated_auxiliary_public_keys::<sha2::Sha256>()
                }

                fn aggregate(signatures: &[[<W3f $key_type Signature>]]) -> Result<[<W3f $key_type Signature>], BlsError> {
                    let mut aggregated_signature =
                        Signature::<$tiny_engine>(<$tiny_engine as EngineBLS>::SignatureGroup::zero());
                    for signature in signatures.iter() {
                        let signature = match tnt_bls::double::DoubleSignature::<$tiny_engine>::from_bytes(
                            &signature.to_bytes()[..],
                        ) {
                            Ok(s) => s,
                            Err(_) => return Err(BlsError::InvalidSignature),
                        };

                        aggregated_signature.0 += signature.0;
                    }

                    let aggregated_signature =
                        match Signature::<$tiny_engine>::from_bytes(&aggregated_signature.to_bytes()) {
                            Ok(sig) => sig,
                            Err(_) => return Err(BlsError::InvalidSignature),
                        };

                    Ok([<W3f $key_type Signature>](aggregated_signature))
                }
            }
        }
    };
}

impl_aggregatable_signature!(Bls377, TinyBLS377);
impl_aggregatable_signature!(Bls381, TinyBLS381);
