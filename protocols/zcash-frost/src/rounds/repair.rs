use dfns_cggmp21::progress::Tracer;
use dfns_cggmp21::round_based::ProtocolMessage;
use frost_core::keys::repairable::{repair_share_step_1, repair_share_step_2, repair_share_step_3};
use frost_core::keys::{SecretShare, VerifiableSecretSharingCommitment};

use frost_core::{Ciphersuite, Field, Group, Identifier, Scalar};
use futures::SinkExt;
use rand_core::{CryptoRng, RngCore};
use round_based::rounds_router::simple_store::RoundInput;
use round_based::rounds_router::RoundsRouter;
use round_based::{Delivery, Mpc, MpcParty, Outgoing};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tangle_primitives::roles::ThresholdSignatureRoleType;

use super::errors::IoError;
use super::{Reason, RepairAborted, RepairError};

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
pub async fn run_threshold_repair<C, R, M>(
    mut tracer: Option<&mut dyn Tracer>,
    i: u16,
    helpers: Vec<u16>,
    share_i: &SecretShare<C>,
    commitment: Option<VerifiableSecretSharingCommitment<C>>,
    participant: u16,
    role: ThresholdSignatureRoleType,
    rng: &mut R,
    party: M,
) -> Result<Option<Vec<u8>>, RepairError<C>>
where
    R: RngCore + CryptoRng,
    M: Mpc<ProtocolMessage = Msg>,
    C: Ciphersuite,
{
    tracer.protocol_begins();

    tracer.stage("Setup networking");
    let MpcParty { delivery, .. } = party.into_party();
    let (incomings, mut outgoings) = delivery.split();

    let mut rounds = RoundsRouter::<Msg>::builder();
    let round1 = rounds.add_round(RoundInput::<MsgRound1>::p2p(i, helpers.len() as u16));
    let round2 = rounds.add_round(RoundInput::<MsgRound2>::broadcast(i, helpers.len() as u16));
    let mut rounds = rounds.listen(incomings);

    // Round 1
    tracer.round_begins();
    let helpers: Vec<Identifier<C>> = helpers
        .iter()
        .map(|i| Identifier::try_from(*i).expect("should be nonzero"))
        .collect();
    let lost_share_participant_identifier =
        Identifier::<C>::try_from(participant).expect("should be nonzero");
    let _my_identifier = Identifier::<C>::try_from(i).expect("should be nonzero");

    tracer.send_msg();
    tracer.stage("Repair share step 1");
    // Calculate the messages to be sent to each party
    let round1_msg_map: BTreeMap<Identifier<C>, Scalar<C>> = helper_round1(
        role,
        &helpers,
        share_i,
        lost_share_participant_identifier,
        rng,
    )?;
    for (identifier, msg) in round1_msg_map {
        let receiver_index_be_bytes: [u8; 2] = identifier
            .serialize()
            .as_ref()
            .try_into()
            .expect("should be 2 bytes");
        let receiver_index = u16::from_be_bytes(receiver_index_be_bytes);
        outgoings
            .send(Outgoing::p2p(
                receiver_index,
                Msg::Round1(MsgRound1 {
                    msg: <C::Group as Group>::Field::serialize(&msg)
                        .as_ref()
                        .to_vec(),
                }),
            ))
            .await
            .map_err(|e| RepairError(Reason::IoError(IoError::send_message(e))))?;
    }
    tracer.msg_sent();

    // Round 2
    tracer.round_begins();

    tracer.receive_msgs();
    let delta_js: Vec<Scalar<C>> = rounds
        .complete(round1)
        .await
        .map_err(|e| RepairError(Reason::IoError(IoError::receive_message(e))))?
        .into_vec_without_me()
        .into_iter()
        .map(|msg| {
            let ser = <<C::Group as Group>::Field as Field>::Serialization::try_from(msg.msg)
                .map_err(|_e| RepairError(Reason::<C>::SerializationError))
                .expect("Failed to deserialize round 1 scalar");
            <C::Group as Group>::Field::deserialize(&ser)
                .unwrap_or(<C::Group as Group>::Field::zero())
        })
        .collect();
    tracer.msgs_received();

    tracer.send_msg();
    tracer.stage("Repair share step 2");
    let round2_msg: Scalar<C> = helper_round2::<C>(role, delta_js.as_ref());
    outgoings
        .send(Outgoing::p2p(
            participant,
            Msg::Round2(MsgRound2 {
                msg: <C::Group as Group>::Field::serialize(&round2_msg)
                    .as_ref()
                    .to_vec(),
            }),
        ))
        .await
        .map_err(|e| RepairError(Reason::IoError(IoError::send_message(e))))?;
    tracer.msg_sent();

    // TODO: Figure out how to properly represent the participant requesting the
    // TODO: share repairing. They do not run `helper_round1` or `helper_round2`.
    // TODO: Instead they just run reconstruct.
    tracer.round_begins();
    tracer.stage("Repair step 3 (run by participant requesting repairing)");
    tracer.receive_msgs();
    let sigmas: Vec<Scalar<C>> = rounds
        .complete(round2)
        .await
        .map_err(|e| RepairError(Reason::IoError(IoError::receive_message(e))))?
        .into_vec_without_me()
        .into_iter()
        .map(|msg| {
            let ser = <<C::Group as Group>::Field as Field>::Serialization::try_from(msg.msg)
                .map_err(|_e| RepairError(Reason::<C>::SerializationError))
                .expect("Failed to deserialize round 1 scalar");
            <C::Group as Group>::Field::deserialize(&ser)
                .unwrap_or(<C::Group as Group>::Field::zero())
        })
        .collect();
    tracer.msgs_received();
    tracer.stage("Repair secret share w/ sigmas from helpers");
    let mut secret_share: Option<Vec<u8>> = None;
    if i == participant {
        let commitment = commitment.unwrap();
        let secret_share_val = repair_round3(
            role,
            &sigmas,
            lost_share_participant_identifier,
            &commitment,
        );
        secret_share = Some(secret_share_val.serialize().unwrap_or_default());
    }
    tracer.protocol_ends();

    Ok(secret_share)
}

/// Step 1 of RTS.
///
/// Generates the "delta" values from `helper_i` to help `participant` recover their share
/// where `helpers` contains the identifiers of all the helpers (including `helper_i`), and `share_i`
/// is the share of `helper_i`.
///
/// Returns a BTreeMap mapping which value should be sent to which participant.
/// Taken from https://github.com/LIT-Protocol/frost/blob/main/frost-ed25519/src/keys/repairable.rs
fn helper_round1<R: RngCore + CryptoRng, C: Ciphersuite>(
    role: ThresholdSignatureRoleType,
    helpers: &[Identifier<C>],
    share_i: &SecretShare<C>,
    participant: Identifier<C>,
    rng: &mut R,
) -> Result<BTreeMap<Identifier<C>, Scalar<C>>, RepairError<C>> {
    match role {
        ThresholdSignatureRoleType::ZcashFrostEd25519
        | ThresholdSignatureRoleType::ZcashFrostP256
        | ThresholdSignatureRoleType::ZcashFrostRistretto255
        | ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {}
        _ => panic!("Invalid role"),
    };

    repair_share_step_1(helpers, share_i, rng, participant).map_err(|e| {
        RepairError(Reason::RepairFailure(RepairAborted::FrostError {
            parties: vec![],
            error: e,
        }))
    })
}

/// Step 2 of RTS.
///
/// Generates the `sigma` values from all `deltas` received from `helpers`
/// to help `participant` recover their share.
/// `sigma` is the sum of all received `delta` and the `delta_i` generated for `helper_i`.
///
/// Returns a scalar
/// Taken from https://github.com/LIT-Protocol/frost/blob/main/frost-ed25519/src/keys/repairable.rs
fn helper_round2<C: Ciphersuite>(
    role: ThresholdSignatureRoleType,
    deltas_j: &[Scalar<C>],
) -> Scalar<C> {
    match role {
        ThresholdSignatureRoleType::ZcashFrostEd25519
        | ThresholdSignatureRoleType::ZcashFrostP256
        | ThresholdSignatureRoleType::ZcashFrostRistretto255
        | ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {}
        _ => panic!("Invalid role"),
    };

    repair_share_step_2::<C>(deltas_j)
}

/// Step 3 of RTS
///
/// The `participant` sums all `sigma_j` received to compute the `share`. The `SecretShare`
/// is made up of the `identifier`and `commitment` of the `participant` as well as the
/// `value` which is the `SigningShare`.
pub fn repair_round3<C: Ciphersuite>(
    role: ThresholdSignatureRoleType,
    sigmas: &[Scalar<C>],
    identifier: Identifier<C>,
    commitment: &VerifiableSecretSharingCommitment<C>,
) -> SecretShare<C> {
    match role {
        ThresholdSignatureRoleType::ZcashFrostEd25519
        | ThresholdSignatureRoleType::ZcashFrostP256
        | ThresholdSignatureRoleType::ZcashFrostRistretto255
        | ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {}
        _ => panic!("Invalid role"),
    };

    repair_share_step_3(sigmas, identifier, commitment)
}
