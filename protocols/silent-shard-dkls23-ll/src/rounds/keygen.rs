use dfns_cggmp21::progress::Tracer;
use dkls23_ll::dkg::{KeygenMsg1, KeygenMsg2, KeygenMsg3, KeygenMsg4, Keyshare, Party, State};
use futures::SinkExt;
use k256::AffinePoint;

use super::{Error, IoError};
use rand_core::{CryptoRng, RngCore};
use round_based::{
    rounds_router::{simple_store::RoundInput, RoundsRouter},
    Delivery, Mpc, MpcParty, Outgoing, ProtocolMessage,
};
use round_based_21 as round_based;
use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! impl_digestable_for_msg_wrapper {
    ($wrapper_type:ty, $msg_type:ident) => {
        impl udigest::Digestable for $wrapper_type {
            fn unambiguously_encode<B: udigest::Buffer>(
                &self,
                encoder: udigest::encoding::EncodeValue<B>,
            ) {
                let msg = &self.0;
                let serialized_msg = serde_json::to_string(msg)
                    .expect(concat!("Failed to serialize ", stringify!($msg_type)));
                encoder.encode_leaf().chain(serialized_msg.as_bytes());
            }
        }

        impl From<$msg_type> for $wrapper_type {
            fn from(msg: $msg_type) -> Self {
                Self(msg)
            }
        }
    };
}

/// Message of key generation protocol
#[derive(ProtocolMessage, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum Msg {
    /// Round 1 message
    Round1(MsgRound1),
    /// Round 1 commitment message
    Round1Comm(MsgRound1Comm),
    /// Round 2 message
    Round2(MsgRound2),
    /// Round 3 message
    Round3(MsgRound3),
    /// Round 4 message
    Round4(MsgRound4),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeygenMsg1Wrapper(pub KeygenMsg1);
impl_digestable_for_msg_wrapper!(KeygenMsg1Wrapper, KeygenMsg1);

/// Message from round 1
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.keygen.threshold.round1")]
pub struct MsgRound1 {
    pub msg: KeygenMsg1Wrapper,
}

/// Message from round 1 commitment phase
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.keygen.threshold.round1_comm")]
pub struct MsgRound1Comm {
    pub msg: [u8; 32],
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeygenMsg2Wrapper(pub KeygenMsg2);
impl_digestable_for_msg_wrapper!(KeygenMsg2Wrapper, KeygenMsg2);

/// Message from round 2
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.keygen.threshold.round2")]
pub struct MsgRound2 {
    pub msg: KeygenMsg2Wrapper,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeygenMsg3Wrapper(pub KeygenMsg3);
impl_digestable_for_msg_wrapper!(KeygenMsg3Wrapper, KeygenMsg3);

/// Message from round 3
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.keygen.threshold.round3")]
pub struct MsgRound3 {
    pub msg: KeygenMsg3Wrapper,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeygenMsg4Wrapper(pub KeygenMsg4);
impl_digestable_for_msg_wrapper!(KeygenMsg4Wrapper, KeygenMsg4);

/// Message from round 4
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.keygen.threshold.round4")]
pub struct MsgRound4 {
    pub msg: KeygenMsg4Wrapper,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SilentShardDKLS23KeyShare {
    pub key_share: Keyshare,
    pub verifying_key: AffinePoint,
}

pub async fn run_threshold_keygen<R, M>(
    mut tracer: Option<&mut dyn Tracer>,
    i: u16,
    t: u16,
    n: u16,
    mut rng: &mut R,
    party: M,
) -> Result<SilentShardDKLS23KeyShare, Error>
where
    R: RngCore + CryptoRng,
    M: Mpc<ProtocolMessage = Msg>,
{
    tracer.protocol_begins();

    tracer.stage("Setup networking");
    let MpcParty { delivery, .. } = party.into_party();
    let (incomings, mut outgoings) = delivery.split();

    let mut rounds = RoundsRouter::<Msg>::builder();
    let round1 = rounds.add_round(RoundInput::<MsgRound1>::broadcast(i, n));
    let round1_comm = rounds.add_round(RoundInput::<MsgRound1Comm>::broadcast(i, n));
    let round2 = rounds.add_round(RoundInput::<MsgRound2>::p2p(i, n));
    let round3 = rounds.add_round(RoundInput::<MsgRound3>::p2p(i, n));
    let round4 = rounds.add_round(RoundInput::<MsgRound4>::broadcast(i, n));
    let mut rounds = rounds.listen(incomings);

    // Round 1
    tracer.round_begins();
    let mut p = State::new(
        Party {
            ranks: vec![0u8; n as usize],
            party_id: i as u8,
            t: t as u8,
        },
        &mut rng,
        None,
    );

    tracer.stage("Compute round 1 dkg secret package");
    let keygen_msg1 = p.generate_msg1();

    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round1(MsgRound1 {
            msg: keygen_msg1.into(),
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    // Round 1 commmitment phase
    // After handling batch msg1, all parties calculate final session id,
    // and now we have to calculate commitments for chain_code_sid
    tracer.round_begins();

    tracer.receive_msgs();
    let round1_packages: Vec<KeygenMsg1> = rounds
        .complete(round1)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me()
        .into_iter()
        .map(|msg| msg.msg.0)
        .collect();
    tracer.msgs_received();

    tracer.stage("Compute round 2 dkg secret package");
    let keygen_msg2: Vec<KeygenMsg2> = p.handle_msg1(&mut rng, round1_packages)?;

    // Round 1 commitment phase
    tracer.stage("Compute round 1 commitment after keygen msg2");
    let commitment = p.calculate_commitment_2();
    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round1Comm(MsgRound1Comm {
            msg: commitment,
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    tracer.receive_msgs();
    let commitments: Vec<[u8; 32]> = rounds
        .complete(round1_comm)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_including_me(MsgRound1Comm { msg: commitment })
        .into_iter()
        .map(|msg| msg.msg)
        .collect();

    // Round 2
    tracer.stage("Send keygen msg2");
    tracer.send_msg();
    for msg in keygen_msg2 {
        outgoings
            .send(Outgoing::p2p(
                msg.to_id as u16,
                Msg::Round2(MsgRound2 { msg: msg.into() }),
            ))
            .await
            .map_err(IoError::send_message)?;
    }
    tracer.msg_sent();

    // Round 3
    tracer.round_begins();

    tracer.receive_msgs();
    let round2_packages: Vec<KeygenMsg2> = rounds
        .complete(round2)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me()
        .into_iter()
        .map(|msg| msg.msg.0)
        .collect();
    tracer.msgs_received();

    tracer.stage("Compute round 3 dkg secret package");
    let keygen_msg3: Vec<KeygenMsg3> = p.handle_msg2(&mut rng, round2_packages)?;

    tracer.send_msg();
    for msg in keygen_msg3 {
        outgoings
            .send(Outgoing::p2p(
                msg.to_id as u16,
                Msg::Round3(MsgRound3 { msg: msg.into() }),
            ))
            .await
            .map_err(IoError::send_message)?;
    }
    tracer.msg_sent();

    // Round 4
    tracer.round_begins();

    tracer.receive_msgs();
    let round3_packages: Vec<KeygenMsg3> = rounds
        .complete(round3)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me()
        .into_iter()
        .map(|msg| msg.msg.0)
        .collect();
    tracer.msgs_received();

    tracer.stage("Compute round 4 dkg secret package");
    let keygen_msg4 = p.handle_msg3(&mut rng, round3_packages, &commitments)?;

    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round4(MsgRound4 {
            msg: keygen_msg4.into(),
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    tracer.receive_msgs();
    let round4_packages: Vec<KeygenMsg4> = rounds
        .complete(round4)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me()
        .into_iter()
        .map(|msg| msg.msg.0)
        .collect();
    tracer.msgs_received();

    let key_share = p.handle_msg4(round4_packages)?;
    tracer.protocol_ends();

    let public_key: AffinePoint = key_share.public_key;
    Ok(SilentShardDKLS23KeyShare {
        key_share,
        verifying_key: public_key,
    })
}
