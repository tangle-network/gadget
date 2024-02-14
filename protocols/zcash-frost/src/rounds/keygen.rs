use std::collections::BTreeMap;

use dfns_cggmp21::progress::Tracer;
use frost_core::{
    keys::{
        dkg::{round1, round2},
        KeyPackage, PublicKeyPackage,
    },
    Ciphersuite, Error, Identifier,
};
use futures::SinkExt;
use rand_core::{CryptoRng, RngCore};
use round_based::{
    rounds_router::simple_store::RoundInput,
    rounds_router::{simple_store::RoundMsgs, RoundsRouter},
    Delivery, Mpc, MpcParty, Outgoing, ProtocolMessage,
};
use serde::{Deserialize, Serialize};
use tangle_primitives::roles::ThresholdSignatureRoleType;

use super::{IoError, KeygenAborted, KeygenError, Reason};

/// Message of key generation protocol
#[derive(ProtocolMessage, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum Msg {
    /// Round 1 message
    Round1(MsgRound1),
    /// Round 2 message
    Round2(MsgRound2),
    /// Round 3 message
    Round3(MsgRound3),
}

/// Message from round 1
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "zcash.frost.keygen.threshold.round1")]
pub struct MsgRound1 {
    pub msg: Vec<u8>,
}
/// Message from round 2
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "zcash.frost.keygen.threshold.round2")]
pub struct MsgRound2 {
    pub msg: Vec<u8>,
}
/// Message from round 3
#[derive(Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct MsgRound3 {
    pub msg: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrostKeyShare {
    pub key_package: Vec<u8>,
    pub pubkey_package: Vec<u8>,
    pub verifying_key: Vec<u8>,
}

pub async fn run_threshold_keygen<C, R, M>(
    mut tracer: Option<&mut dyn Tracer>,
    i: u16,
    t: u16,
    n: u16,
    role: ThresholdSignatureRoleType,
    rng: &mut R,
    party: M,
) -> Result<FrostKeyShare, KeygenError<C>>
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
    let round1 = rounds.add_round(RoundInput::<MsgRound1>::broadcast(i, n));
    let round2 = rounds.add_round(RoundInput::<MsgRound2>::p2p(i, n));
    let mut rounds = rounds.listen(incomings);

    // Round 1
    tracer.round_begins();
    tracer.stage("Compute round 1 dkg secret package");
    let (round1_secret_package, round1_package) =
        dkg_part1(i + 1, t, n, role, rng).map_err(|e| {
            KeygenError(Reason::KeygenFailure(KeygenAborted::FrostError {
                parties: vec![],
                error: e,
            }))
        })?;

    tracer.send_msg();
    let my_round1_msg = MsgRound1 {
        msg: round1_package.serialize().unwrap_or_default(),
    };
    outgoings
        .send(Outgoing::broadcast(Msg::Round1(my_round1_msg.clone())))
        .await
        .map_err(|e| KeygenError(Reason::IoError(IoError::send_message(e))))?;
    tracer.msg_sent();

    // Round 2
    tracer.round_begins();

    tracer.receive_msgs();
    let round1_packages: Vec<round1::Package<C>> = rounds
        .complete(round1)
        .await
        .map_err(|e| KeygenError(Reason::IoError(IoError::receive_message(e))))?
        .into_vec_including_me(my_round1_msg.clone())
        .into_iter()
        .map(|msg| {
            round1::Package::deserialize(&msg.msg)
                .unwrap_or_else(|_| panic!("Failed to deserialize round 1 package"))
        })
        .collect();
    tracer.msgs_received();
    tracer.stage("Compute round 2 dkg secret package");
    let round1_packages_map: BTreeMap<Identifier<C>, round1::Package<C>> = round1_packages
        .iter()
        .enumerate()
        .filter(|(inx, _)| *inx != i as usize)
        .map(|(inx, p)| {
            (
                ((inx + 1) as u16).try_into().expect("should be nonzero"),
                p.clone(),
            )
        })
        .collect();
    let (round2_secret_package, round2_packages_map) =
        dkg_part2(role, round1_secret_package, &round1_packages_map)?;

    tracer.send_msg();
    for (receiver_identifier, round2_package) in round2_packages_map {
        let receiver_index_bytes: Vec<u8> = receiver_identifier.serialize().as_ref().to_vec();
        let receiver_index = u16::from_le_bytes([receiver_index_bytes[0], receiver_index_bytes[1]]);
        outgoings
            .send(Outgoing::p2p(
                receiver_index - 1,
                Msg::Round2(MsgRound2 {
                    msg: round2_package.serialize().unwrap_or_default(),
                }),
            ))
            .await
            .map_err(IoError::send_message)?;
    }
    tracer.msg_sent();

    // Round 3
    tracer.round_begins();

    tracer.receive_msgs();
    let round2_packages: RoundMsgs<MsgRound2> = rounds
        .complete(round2)
        .await
        .map_err(IoError::receive_message)?;
    tracer.msgs_received();

    tracer.stage("Compute round 3 dkg secret package");
    let round2_packages_map: BTreeMap<Identifier<C>, round2::Package<C>> = round2_packages
        .into_vec_including_me(MsgRound2 { msg: vec![] })
        .into_iter()
        .enumerate()
        .filter(|(inx, _)| *inx != i as usize)
        .map(|(inx, msg)| {
            let identifier = (inx as u16 + 1).try_into().expect("should be nonzero");
            let package = round2::Package::deserialize(&msg.msg)
                .unwrap_or_else(|_| panic!("Failed to deserialize round 2 package"));
            (identifier, package)
        })
        .collect();
    let (key_package, pubkey_package) = dkg_part3(
        role,
        &round2_secret_package,
        &round1_packages_map,
        &round2_packages_map,
    )?;

    tracer.protocol_ends();
    Ok(FrostKeyShare {
        key_package: key_package.serialize().unwrap_or_default(),
        pubkey_package: pubkey_package.serialize().unwrap_or_default(),
        verifying_key: pubkey_package.verifying_key().serialize().as_ref().to_vec(),
    })
}

pub fn dkg_part1<R, C>(
    i: u16,
    t: u16,
    n: u16,
    role: ThresholdSignatureRoleType,
    rng: R,
) -> Result<(round1::SecretPackage<C>, round1::Package<C>), Error<C>>
where
    R: RngCore + CryptoRng,
    C: Ciphersuite,
{
    match role {
        ThresholdSignatureRoleType::ZcashFrostEd25519
        | ThresholdSignatureRoleType::ZcashFrostP256
        | ThresholdSignatureRoleType::ZcashFrostRistretto255
        | ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {}
        _ => panic!("Invalid role"),
    };

    let participant_identifier = i.try_into().expect("should be nonzero");
    frost_core::keys::dkg::part1::<C, R>(participant_identifier, n, t, rng)
}

#[allow(clippy::type_complexity)]
pub fn dkg_part2<C>(
    role: ThresholdSignatureRoleType,
    secret_package: round1::SecretPackage<C>,
    round1_packages: &BTreeMap<Identifier<C>, round1::Package<C>>,
) -> Result<
    (
        round2::SecretPackage<C>,
        BTreeMap<Identifier<C>, round2::Package<C>>,
    ),
    Error<C>,
>
where
    C: Ciphersuite,
{
    match role {
        ThresholdSignatureRoleType::ZcashFrostEd25519
        | ThresholdSignatureRoleType::ZcashFrostP256
        | ThresholdSignatureRoleType::ZcashFrostRistretto255
        | ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {}
        _ => panic!("Invalid role"),
    };

    frost_core::keys::dkg::part2::<C>(secret_package, round1_packages)
}

pub fn dkg_part3<C>(
    role: ThresholdSignatureRoleType,
    round2_secret_package: &round2::SecretPackage<C>,
    round1_packages: &BTreeMap<Identifier<C>, round1::Package<C>>,
    round2_packages: &BTreeMap<Identifier<C>, round2::Package<C>>,
) -> Result<(KeyPackage<C>, PublicKeyPackage<C>), Error<C>>
where
    C: Ciphersuite,
{
    match role {
        ThresholdSignatureRoleType::ZcashFrostEd25519
        | ThresholdSignatureRoleType::ZcashFrostP256
        | ThresholdSignatureRoleType::ZcashFrostRistretto255
        | ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {}
        _ => panic!("Invalid role"),
    };

    frost_core::keys::dkg::part3::<C>(round2_secret_package, round1_packages, round2_packages)
}
