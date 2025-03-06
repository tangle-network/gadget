use crate::{bls::*, error::SpCoreError};
use gadget_crypto_core::{aggregation::AggregatableSignature, KeyType};
use gadget_std::Zero;
use sp_core::ByteArray;
use tnt_bls::{single_pop_aggregator::SignatureAggregatorAssumingPoP, *};

macro_rules! impl_aggregatable_signature {
    ($key_type:ident, $tiny_engine:ty) => {
        paste::paste! {
            impl AggregatableSignature for [<Sp $key_type>] {
                fn verify_single(
                    message: &[u8],
                    signature: &<Self as KeyType>::Signature,
                    public_key: &<Self as KeyType>::Public,
                ) -> bool {
                    <Self>::verify(public_key, message, signature)
                }

                fn verify_aggregate(
                    message: &[u8],
                    signature: &<Self as KeyType>::Signature,
                    public_keys: &[<Self as KeyType>::Public],
                ) -> bool {
                    let sig = match tnt_bls::Signature::<$tiny_engine>::from_bytes(&signature.0[..]) {
                        Ok(s) => s,
                        Err(_) => return false,
                    };
                    let mut aggregated_public_key =
                        PublicKey::<$tiny_engine>(<$tiny_engine as EngineBLS>::PublicKeyGroup::zero());

                    let mut double_pubkeys: Vec<DoublePublicKey<$tiny_engine>> = vec![];

                    for key in public_keys {
                        let public_key =
                            match tnt_bls::double::DoublePublicKey::<$tiny_engine>::from_bytes(
                                &key.0 .0,
                            ) {
                                Ok(pk) => pk,
                                Err(_) => return false,
                            };
                        aggregated_public_key.0 += public_key.1.clone();
                        double_pubkeys.push(public_key);
                    }

                    let public_keys_in_sig_grp: Vec<PublicKeyInSignatureGroup<$tiny_engine>> =
                        double_pubkeys
                            .iter()
                            .map(|k| PublicKeyInSignatureGroup(k.0))
                            .collect();

                    let message: Message = Message::new(b"", message);
                    let mut prover_aggregator =
                        SignatureAggregatorAssumingPoP::<$tiny_engine>::new(message.clone());
                    prover_aggregator.add_signature(&sig);
                    let mut verifier_aggregator =
                        SignatureAggregatorAssumingPoP::<$tiny_engine>::new(message);
                    //get the signature and already aggregated public key from the prover
                    verifier_aggregator.add_signature(&(&prover_aggregator).signature());
                    verifier_aggregator.add_publickey(&aggregated_public_key);

                    // aggregate public keys in signature group
                    public_keys_in_sig_grp.iter().for_each(|pk| {
                        verifier_aggregator.add_auxiliary_public_key(pk);
                    });

                    verifier_aggregator.verify_using_aggregated_auxiliary_public_keys::<sha2::Sha256>()
                }

                fn aggregate(
                    signatures: &[<Self as KeyType>::Signature],
                ) -> Result<<Self as KeyType>::Signature, SpCoreError> {
                    let mut aggregated_signature =
                        Signature::<$tiny_engine>(<$tiny_engine as EngineBLS>::SignatureGroup::zero());
                    for signature in signatures.iter() {
                        let signature =
                            match tnt_bls::double::DoubleSignature::<$tiny_engine>::from_bytes(
                                &signature.0[..],
                            ) {
                                Ok(s) => s,
                                Err(_) => return Err(SpCoreError::InvalidSignature),
                            };

                        aggregated_signature.0 += signature.0;
                    }

                    let aggregated_signature =
                        match sp_core::bls::Signature::from_slice(&aggregated_signature.to_bytes()) {
                            Ok(sig) => sig,
                            Err(_) => return Err(SpCoreError::InvalidSignature),
                        };

                    Ok([<Sp $key_type Signature>](aggregated_signature))
                }
            }
        }
    };
}

impl_aggregatable_signature!(Bls377, TinyBLS377);
impl_aggregatable_signature!(Bls381, TinyBLS381);
