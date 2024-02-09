use dfns_cggmp21::progress::Tracer;
use frost_core::keys::{KeyPackage, PublicKeyPackage};
use frost_core::round1::{SigningCommitments, SigningNonces};
use frost_core::round2::{self, SignatureShare};
use frost_core::{aggregate, round1, Ciphersuite, Field, Group, Identifier, SigningPackage};
use futures::SinkExt;
use rand_core::{CryptoRng, RngCore};
use round_based::rounds_router::simple_store::RoundInput;

use round_based::rounds_router::RoundsRouter;
use round_based::ProtocolMessage;
use round_based::{Delivery, Mpc, MpcParty, Outgoing};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tangle_primitives::roles::ThresholdSignatureRoleType;

use super::errors::IoError;
use super::{Reason, SignAborted, SignError};

/// Message of key generation protocol
#[derive(ProtocolMessage, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum Msg {
    /// Round 1 message
    Round1(MsgRound1),
    /// Round 2 message
    Round2(MsgRound2),
}

/// Message from round 1
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "zcash.frost.sign.threshold.round1")]
pub struct MsgRound1 {
    pub msg: Vec<u8>,
}
/// Message from round 2
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "zcash.frost.sign.threshold.round2")]
pub struct MsgRound2 {
    pub msg: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrostSignature {
    pub group_signature: Vec<u8>,
}

#[allow(clippy::too_many_arguments)]
pub async fn run_threshold_sign<C, R, M>(
    mut tracer: Option<&mut dyn Tracer>,
    i: u16,
    signers: Vec<u16>,
    frost_keyshare: (KeyPackage<C>, PublicKeyPackage<C>),
    message_to_sign: &[u8],
    role: ThresholdSignatureRoleType,
    rng: &mut R,
    party: M,
) -> Result<FrostSignature, SignError<C>>
where
    R: RngCore + CryptoRng,
    M: Mpc<ProtocolMessage = Msg>,
    C: Ciphersuite,
{
    tracer.protocol_begins();

    tracer.stage("Setup networking");
    let MpcParty { delivery, .. } = party.into_party();
    let (incomings, mut outgoings) = delivery.split();
    println!("Signers: {:?}", signers);
    let mut rounds = RoundsRouter::<Msg>::builder();
    let round1 = rounds.add_round(RoundInput::<MsgRound1>::broadcast(i, signers.len() as u16));
    let round2 = rounds.add_round(RoundInput::<MsgRound2>::broadcast(i, signers.len() as u16));
    let mut rounds = rounds.listen(incomings);

    // Round 1
    tracer.round_begins();

    tracer.send_msg();
    tracer.stage("Generate nonces and commitments for Round 1");
    let (nonces, commitments) = participant_round1(role, &frost_keyshare.0, rng);
    let my_round1_msg = MsgRound1 {
        msg: commitments.serialize().unwrap_or_default(),
    };
    outgoings
        .send(Outgoing::broadcast(Msg::Round1(my_round1_msg.clone())))
        .await
        .map_err(|e| SignError(Reason::IoError(IoError::send_message(e))))?;
    tracer.msg_sent();

    // Round 2
    tracer.round_begins();

    tracer.receive_msgs();
    let round1_msgs: Vec<MsgRound1> = rounds
        .complete(round1)
        .await
        .map_err(|e| SignError(Reason::IoError(IoError::receive_message(e))))?
        .into_vec_including_me(my_round1_msg);

    let round1_signing_commitments: BTreeMap<Identifier<C>, SigningCommitments<C>> = round1_msgs
        .into_iter()
        .enumerate()
        .map(|(party_inx, msg)| {
            let msg = SigningCommitments::<C>::deserialize(&msg.msg)
                .unwrap_or_else(|_| panic!("Failed to deserialize round 1 signing commitments"));
            let participant_identifier = Identifier::<C>::try_from((party_inx + 1) as u16)
                .expect("Failed to convert party index to identifier");
            (participant_identifier, msg)
        })
        .collect();
    tracer.msgs_received();

    tracer.send_msg();
    tracer.stage(
        "Produce signature share using the `SigningPackage` and `SigningNonces` from Round 1",
    );
    let signing_package = SigningPackage::<C>::new(round1_signing_commitments, message_to_sign);
    let signature_share = participant_round2(role, &signing_package, &nonces, &frost_keyshare.0)?;
    outgoings
        .send(Outgoing::broadcast(Msg::Round2(MsgRound2 {
            msg: signature_share.serialize().as_ref().to_vec(),
        })))
        .await
        .map_err(|e| SignError(Reason::IoError(IoError::send_message(e))))?;
    tracer.msg_sent();

    // Aggregation / output round
    tracer.round_begins();

    tracer.receive_msgs();
    let round2_signature_shares: BTreeMap<Identifier<C>, SignatureShare<C>> = rounds
        .complete(round2)
        .await
        .map_err(|e| SignError(Reason::IoError(IoError::receive_message(e))))?
        .into_vec_including_me(MsgRound2 {
            msg: signature_share.serialize().as_ref().to_vec(),
        })
        .into_iter()
        .enumerate()
        .map(|(party_inx, msg)| {
            let participant_identifier = Identifier::<C>::try_from((party_inx + 1) as u16)
                .expect("Failed to convert party index to identifier");
            let ser = <<C::Group as Group>::Field as Field>::Serialization::try_from(msg.msg)
                .map_err(|_e| SignError(Reason::<C>::SerializationError))
                .expect("Failed to deserialize round 2 signature share");
            let sig_share = SignatureShare::<C>::deserialize(ser)
                .unwrap_or_else(|_| panic!("Failed to deserialize round 2 signature share"));
            (participant_identifier, sig_share)
        })
        .collect();
    tracer.msgs_received();

    let group_signature = aggregate(
        &signing_package,
        &round2_signature_shares,
        &frost_keyshare.1,
    )
    .map_err(|e| {
        SignError(Reason::SignFailure(SignAborted::FrostError {
            parties: vec![],
            error: e,
        }))
    })?;

    if frost_keyshare
        .1
        .verifying_key()
        .verify(message_to_sign, &group_signature)
        .is_err()
    {
        return Err(SignError(Reason::SignFailure(SignAborted::FrostError {
            parties: vec![],
            error: frost_core::Error::<C>::InvalidSignature,
        })));
    }

    tracer.protocol_ends();

    Ok(FrostSignature {
        group_signature: group_signature.serialize().as_ref().to_vec(),
    })
}

/// Participant generates nonces and commitments for Round 1.
fn participant_round1<R: RngCore + CryptoRng, C: Ciphersuite>(
    role: ThresholdSignatureRoleType,
    key_package: &KeyPackage<C>,
    rng: &mut R,
) -> (SigningNonces<C>, SigningCommitments<C>) {
    match role {
        ThresholdSignatureRoleType::ZcashFrostEd25519
        | ThresholdSignatureRoleType::ZcashFrostP256
        | ThresholdSignatureRoleType::ZcashFrostRistretto255
        | ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {}
        _ => panic!("Invalid role"),
    };

    round1::commit(key_package.signing_share(), rng)
}

/// Participant produces their signature share using the `SigningPackage` and their `SigningNonces` from Round 1.
fn participant_round2<C: Ciphersuite>(
    role: ThresholdSignatureRoleType,
    signing_package: &SigningPackage<C>,
    nonces: &SigningNonces<C>,
    key_package: &KeyPackage<C>,
) -> Result<SignatureShare<C>, SignError<C>> {
    match role {
        ThresholdSignatureRoleType::ZcashFrostEd25519
        | ThresholdSignatureRoleType::ZcashFrostP256
        | ThresholdSignatureRoleType::ZcashFrostRistretto255
        | ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {}
        _ => panic!("Invalid role"),
    };

    println!("Min signers: {:?}", key_package.min_signers());
    println!(
        "Signing package commits: {:?}",
        signing_package.signing_commitments().len()
    );
    round2::sign(signing_package, nonces, key_package).map_err(|e| {
        SignError(Reason::SignFailure(SignAborted::FrostError {
            parties: vec![],
            error: e,
        }))
    })
}
