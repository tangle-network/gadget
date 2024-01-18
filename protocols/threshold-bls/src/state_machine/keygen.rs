use bls12_381_plus::{group::GroupEncoding, G1Projective, Scalar};
use dfns_cggmp21::{progress::Tracer, security_level::SecurityLevel, ExecutionId};
use digest::Digest;
use futures::SinkExt;
use gennaro_dkg::{
    rand_core::{CryptoRng, RngCore},
    Error, Parameters, Round1BroadcastData, Round1P2PData, Round2EchoBroadcastData,
    Round3BroadcastData, Round4EchoBroadcastData, SecretParticipant,
};
use round_based::{
    rounds_router::simple_store::{RoundInput, RoundMsgs},
    rounds_router::{errors::IoError, RoundsRouter},
    Delivery, Mpc, MpcParty, Outgoing, ProtocolMessage,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroUsize};
use vsss_rs::{
    combine_shares,
    elliptic_curve::{hash2curve::GroupDigest, Curve, Group, PrimeField},
    Share,
};

/// Message of key generation protocol
#[derive(ProtocolMessage, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum Msg<G, D>
where
    G: Group + GroupEncoding + Default,
    D: Digest,
{
    Round1Broadcast(MsgRound1Broadcast<G>),
    Round1P2P(MsgRound1P2P),
    Round2(MsgRound2),
    Round3(MsgRound3<G>),
    Round4(MsgRound4<G>),
    /// Reliability check message (optional additional round)
    ReliabilityCheck(MsgReliabilityCheck<D>),
}

/// Message from round 1
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "gennaro.dkg.keygen.round1")]
pub struct MsgRound1Broadcast<G: Group + GroupEncoding + Default> {
    #[serde(with = "hex::serde")]
    #[udigest(as_bytes)]
    pub msg: Round1BroadcastData<G>,
}

/// Message from round 1 that should only be sent to a specific secret_participant
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "gennaro.dkg.keygen.round1p2p")]
pub struct MsgRound1P2P {
    #[serde(with = "hex::serde")]
    #[udigest(as_bytes)]
    pub msg: Round1P2PData,
}

/// Message from round 2
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "gennaro.dkg.keygen.round2")]
pub struct MsgRound2 {
    #[serde(with = "hex::serde")]
    #[udigest(as_bytes)]
    pub msg: Round2EchoBroadcastData,
}

/// Message from round 3
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "gennaro.dkg.keygen.round3")]
pub struct MsgRound3<G: Group + GroupEncoding + Default> {
    #[serde(with = "hex::serde")]
    #[udigest(as_bytes)]
    pub msg: Round3BroadcastData<G>,
}

/// Message from round 4
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "gennaro.dkg.keygen.round4")]
pub struct MsgRound4<G: Group + GroupEncoding + Default> {
    #[serde(with = "hex::serde")]
    #[udigest(as_bytes)]
    pub msg: Round4EchoBroadcastData<G>,
}

/// Message parties exchange to ensure reliability of broadcast channel
#[derive(Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct MsgReliabilityCheck<D: Digest>(pub digest::Output<D>);

#[derive(udigest::Digestable)]
#[udigest(tag = "gennaro.dkg.keygen.tag")]
enum Tag<'a> {
    /// Tag that includes the prover index
    Indexed {
        party_index: u16,
        #[udigest(as_bytes)]
        sid: &'a [u8],
    },
    /// Tag w/o party index
    Unindexed {
        #[udigest(as_bytes)]
        sid: &'a [u8],
    },
}

pub async fn run_threshold_keygen<G, R, M, L, D>(
    mut tracer: Option<&mut dyn Tracer>,
    i: u16,
    t: u16,
    n: u16,
    reliable_broadcast_enforced: bool,
    execution_id: ExecutionId<'_>,
    rng: &mut R,
    party: M,
) -> Result<(), Error>
where
    G: Group + GroupEncoding + Default,
    D: Digest + Clone + 'static,
    R: RngCore + CryptoRng,
    M: Mpc<ProtocolMessage = Msg<G, D>>,
{
    tracer.protocol_begins();
    let parameters = Parameters::new(
        NonZeroUsize::new(t.into()).unwrap(),
        NonZeroUsize::new(n.into()).unwrap(),
    );
    let mut participant =
        SecretParticipant::<G>::new(NonZeroUsize::new(1).unwrap(), parameters).unwrap();

    tracer.stage("Setup networking");
    let MpcParty { delivery, .. } = party.into_party();
    let (incomings, mut outgoings) = delivery.split();

    let mut rounds = RoundsRouter::<Msg<G, D>>::builder();
    let round1_broadcast = rounds.add_round(RoundInput::<MsgRound1Broadcast<G>>::broadcast(i, n));
    let round1_p2p = rounds.add_round(RoundInput::<MsgRound1P2P>::p2p(i, n));
    let round2 = rounds.add_round(RoundInput::<MsgRound2>::broadcast(i, n));
    let round3 = rounds.add_round(RoundInput::<MsgRound3<G>>::p2p(i, n));
    let round4 = rounds.add_round(RoundInput::<MsgRound4<G>>::broadcast(i, n));
    let mut rounds = rounds.listen(incomings);

    // Round 1
    tracer.round_begins();

    tracer.send_msg();
    let (round1_broadcast_msg, round1_p2p_msgs): (
        Round1BroadcastData<G>,
        BTreeMap<usize, Round1P2PData>,
    ) = match participant.round1() {
        Ok(msgs) => msgs,
        Err(e) => {
            return Err(Error::RoundError(
                1,
                "Failed to create round 1 broadcast message".to_string(),
            ))
        }
    };
    outgoings
        .send(Outgoing::broadcast(Msg::Round1Broadcast(
            MsgRound1Broadcast {
                msg: round1_broadcast_msg,
            },
        )))
        .await
        .map_err(|e| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;
    for (j, msg) in round1_p2p_msgs.into_iter() {
        outgoings
            .send(Outgoing::p2p(
                u16::try_from(j).unwrap_or_default(),
                Msg::Round1P2P(MsgRound1P2P { msg }),
            ))
            .await
            .map_err(|e| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;
    }
    assert!(participant.round1().is_err());
    tracer.msg_sent();

    // Round 2
    tracer.round_begins();
    tracer.receive_msgs();
    let round1_broadcast_msgs: RoundMsgs<MsgRound1Broadcast<G>> =
        rounds.complete(round1_broadcast).await.map_err(|e| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;
    let round1_p2p_msgs: RoundMsgs<MsgRound1P2P> =
        rounds.complete(round1_p2p).await.map_err(|e| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;
    tracer.msgs_received();
    tracer.send_msg();
    let round1_broadcast_msg_map: BTreeMap<usize, Round1BroadcastData<G>> = round1_broadcast_msgs
        .into_iter_indexed()
        .map(|(j, _, msg)| (usize::from(j), msg.msg))
        .collect();
    let round1_p2p_msg_map: BTreeMap<usize, Round1P2PData> = round1_p2p_msgs
        .into_iter_indexed()
        .map(|(j, _, msg)| (usize::from(j), msg.msg))
        .collect();
    let round2_msg: Round2EchoBroadcastData =
        match participant.round2(round1_broadcast_msg_map, round1_p2p_msg_map) {
            Ok(msg) => msg,
            Err(e) => {
                return Err(Error::RoundError(
                    2,
                    format!("Failed to create round 2 message: {}", e),
                ))
            }
        };
    outgoings
        .send(Outgoing::broadcast(Msg::Round2(MsgRound2 {
            msg: round2_msg,
        })))
        .await
        .map_err(|e| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;
    tracer.msg_sent();

    // Round 3
    tracer.round_begins();
    tracer.receive_msgs();
    let round2_msgs: RoundMsgs<MsgRound2> = rounds.complete(round2).await.map_err(|e| {
        Error::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        ))
    })?;
    let mut round2_msg_map: BTreeMap<usize, Round2EchoBroadcastData> = round2_msgs
        .into_iter_indexed()
        .map(|(j, _, msg)| (usize::from(j), msg.msg))
        .collect();
    // Add our own message into map
    round2_msg_map.insert(usize::from(i), round2_msg);
    tracer.msgs_received();

    tracer.send_msg();
    let round3_msg: Round3BroadcastData<G> = match participant.round3(&round2_msg_map) {
        Ok(msg) => msg,
        Err(e) => {
            return Err(Error::RoundError(
                3,
                format!("Failed to create round 3 message: {}", e),
            ))
        }
    };
    outgoings
        .send(Outgoing::broadcast(Msg::Round3(MsgRound3 {
            msg: round3_msg,
        })))
        .await
        .map_err(|e| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;
    tracer.msg_sent();

    // Round 4
    tracer.round_begins();
    tracer.receive_msgs();
    tracer.msgs_received();
    tracer.send_msg();
    tracer.msg_sent();

    // Output round
    tracer.round_begins();
    tracer.receive_msgs();
    tracer.msgs_received();
    tracer.protocol_ends();

    Ok(())
}
