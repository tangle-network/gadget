use std::collections::BTreeMap;

use dfns_cggmp21::progress::Tracer;
use dkls23_ll::dkg::{KeygenMsg1, KeygenMsg2, KeygenMsg3, KeygenMsg4, Party, State, Keyshare};
use futures::SinkExt;
use k256::AffinePoint;
use rand_core::{CryptoRng, RngCore};
use round_based::{
    rounds_router::{
        simple_store::{RoundInput, RoundMsgs},
        RoundsRouter,
    },
    runtime::AsyncRuntime,
    Delivery, Mpc, MpcParty, Outgoing, ProtocolMessage,
};
use round_based_21 as round_based;
use serde::{Deserialize, Serialize};
use tangle_primitives::roles::ThresholdSignatureRoleType;

use super::{Error, IoError};

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
    /// Round 3 commitment message
    Round3Comm(MsgRound3Comm),
    /// Round 4 message
    Round4(MsgRound4),
}

/// Message from round 1
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.keygen.threshold.round1")]
pub struct MsgRound1 {
    pub msg: KeygenMsg1,
}
/// Message from round 2
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.keygen.threshold.round2")]
pub struct MsgRound2 {
    pub msg: KeygenMsg2,
}
/// Message from round 3
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.keygen.threshold.round3")]
pub struct MsgRound3 {
    pub msg: KeygenMsg3,
}
/// Message from round 3 commitment phase
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.keygen.threshold.round3_comm")]
pub struct MsgRound3Comm {
    pub msg: [u8; 32],
}
/// Message from round 4
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.keygen.threshold.round4")]
pub struct MsgRound4 {
    pub msg: KeygenMsg4,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SilentShardDKLS23KeyShare {
    pub key_share: Keyshare,
    pub verifying_key: AffinePoint,
}

pub async fn run_threshold_keygen<R, M>(
    mut tracer: Option<&mut dyn Tracer>,
    i: u16,
    t: u16,
    n: u16,
    role: ThresholdSignatureRoleType,
    rng: &mut R,
    party: M,
) -> Result<SilentShardDKLS23KeyShare, Error>
where
    R: RngCore + CryptoRng,
{
    tracer.protocol_begins();

    tracer.stage("Setup networking");
    let MpcParty {
        delivery, runtime, ..
    } = party.into_party();
    let (incomings, mut outgoings) = delivery.split();

    let mut rounds = RoundsRouter::<Msg>::builder();
    let round1 = rounds.add_round(RoundInput::<MsgRound1>::broadcast(i, n));
    let round2 = rounds.add_round(RoundInput::<MsgRound2>::broadcast(i, n));
    let round3 = rounds.add_round(RoundInput::<MsgRound3>::broadcast(i, n));
    let round3_comm = rounds.add_round(RoundInput::<MsgRound3Comm>::broadcast(i, n));
    let round4 = rounds.add_round(RoundInput::<MsgRound4>::broadcast(i, n));
    let mut rounds = rounds.listen(incomings);

    // Round 1
    tracer.round_begins();
    let p = State::new(
        Party {
            ranks: vec![0u8; n as usize],
            party_id: i,
            t,
        },
        &mut rng,
        None,
    );

    tracer.stage("Compute round 1 dkg secret package");
    let keygen_msg1 = p.generate_msg1()?;
    runtime.yield_now().await;

    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round1(MsgRound1 {
            msg: keygen_msg1,
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    // Round 2
    tracer.round_begins();

    tracer.receive_msgs();
    let round1_packages: Vec<KeygenMsg1> = rounds
        .complete(round1)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me();
    tracer.msgs_received();

    tracer.stage("Compute round 2 dkg secret package");
    let keygen_msg2 = p.handle_msg1(&mut rng, &round1_packages)?;

    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round2(MsgRound2 {
            msg: keygen_msg2,
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    // Round 3
    tracer.round_begins();

    tracer.receive_msgs();
    let round2_packages: Vec<KeygenMsg2> = rounds
        .complete(round2)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me();
    tracer.msgs_received();

    tracer.stage("Compute round 3 dkg secret package");
    let keygen_msg3 = party.handle_msg2(&mut rng, &round2_packages)?;

    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round3(MsgRound3 {
            msg: keygen_msg3,
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    // Round 4
    tracer.round_begins();

    tracer.receive_msgs();
    let round3_packages: Vec<KeygenMsg3> = rounds
        .complete(round3)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me();
    tracer.msgs_received();

    
    tracer.stage("Compute round 3 commitment");
    let keygen_msg3_comm = p.calculate_commitment_2();

    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round3Comm(MsgRound3Comm {
            msg: keygen_msg3_comm,
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    tracer.receive_msgs();
    let round3_comm: Vec<[u8; 32]> = rounds
        .complete(round3_comm)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me();

    tracer.stage("Compute round 4 dkg secret package");
    let keygen_msg4 = p.handle_msg3(&mut rng, &round3_packages, &round3_comm)?;

    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round4(MsgRound4 {
            msg: keygen_msg4,
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    tracer.receive_msgs();
    let round4_packages: Vec<KeygenMsg4> = rounds
        .complete(round4)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me();
    tracer.msgs_received();

    let key_share = p.handle_msg4(&round4_packages)?;


    tracer.protocol_ends();
    Ok(SilentShardDKLS23KeyShare {
        key_share,
        verifying_key: key_share.public_key,
    })
}
