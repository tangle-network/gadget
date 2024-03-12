use derivation_path::DerivationPath;
use dfns_cggmp21::progress::Tracer;
use dkls23_ll::dsg::{
    combine_signatures, create_partial_signature, PartialSignature, PreSignature, SignError,
    SignMsg1, SignMsg2, SignMsg3, SignMsg4, State,
};
use futures::SinkExt;
use k256::ecdsa::Signature;
use rand_core::{CryptoRng, RngCore};

use round_based::rounds_router::simple_store::RoundInput;
use round_based::rounds_router::RoundsRouter;
use round_based::runtime::AsyncRuntime;
use round_based::ProtocolMessage;
use round_based::{Delivery, Mpc, MpcParty, Outgoing};
use round_based_21 as round_based;
use serde::{Deserialize, Serialize};
use sp_core::keccak_256;

use crate::impl_digestable_for_msg_wrapper;

use super::keygen::SilentShardDKLS23KeyShare;
use super::{Error, IoError};

/// Message of DKLS23 threshold ECDSA signing protocol
#[derive(ProtocolMessage, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum Msg {
    /// Round 1 message
    Round1(MsgRound1),
    /// Round 2 message
    Round2(MsgRound2),
    /// Round 3 message
    Round3(MsgRound3),
    /// Round 4 message
    Round4(MsgRound4),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SignMsg1Wrapper(pub SignMsg1);
impl_digestable_for_msg_wrapper!(SignMsg1Wrapper, SignMsg1);

/// Message from round 1
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.sign.threshold.round1")]
pub struct MsgRound1 {
    pub msg: SignMsg1Wrapper,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SignMsg2Wrapper(pub SignMsg2);
impl_digestable_for_msg_wrapper!(SignMsg2Wrapper, SignMsg2);

/// Message from round 2
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.sign.threshold.round2")]
pub struct MsgRound2 {
    pub msg: SignMsg2Wrapper,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SignMsg3Wrapper(pub SignMsg3);
impl_digestable_for_msg_wrapper!(SignMsg3Wrapper, SignMsg3);

/// Message from round 3
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.sign.threshold.round3")]
pub struct MsgRound3 {
    pub msg: SignMsg3Wrapper,
}

#[derive(Serialize, Deserialize)]
pub struct PreSignatureWrapper(pub PreSignature);
impl_digestable_for_msg_wrapper!(PreSignatureWrapper, PreSignature);

impl Clone for PreSignatureWrapper {
    fn clone(&self) -> Self {
        Self(PreSignature {
            from_id: self.0.from_id,
            final_session_id: self.0.final_session_id,
            public_key: self.0.public_key,
            s_0: self.0.s_0,
            s_1: self.0.s_1,
            r: self.0.r,
            phi_i: self.0.phi_i,
        })
    }
}

/// Message from round 4
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "silent.shard.dkls23.sign.threshold.round4")]
pub struct MsgRound4 {
    pub msg: PreSignatureWrapper,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SilentSharedDKLS23EcdsaSignature {
    pub group_signature: Signature,
}

#[allow(clippy::too_many_arguments)]
pub async fn run_threshold_sign<R, M>(
    mut tracer: Option<&mut dyn Tracer>,
    i: u16,
    signers: Vec<u16>,
    dkls_keyshare: SilentShardDKLS23KeyShare,
    derivation_path: DerivationPath,
    pre_hashed_msg: &[u8],
    rng: &mut R,
    party: M,
) -> Result<SilentSharedDKLS23EcdsaSignature, Error>
where
    R: RngCore + CryptoRng,
    M: Mpc<ProtocolMessage = Msg>,
{
    let pre_hashed_msg = if pre_hashed_msg.len() != 32 {
        keccak_256(pre_hashed_msg)
    } else {
        pre_hashed_msg.try_into().unwrap()
    };
    tracer.protocol_begins();

    tracer.stage("Setup networking");
    let MpcParty {
        delivery, runtime, ..
    } = party.into_party();
    let (incomings, mut outgoings) = delivery.split();
    let mut rounds = RoundsRouter::<Msg>::builder();
    let round1 = rounds.add_round(RoundInput::<MsgRound1>::broadcast(i, signers.len() as u16));
    let round2 = rounds.add_round(RoundInput::<MsgRound2>::p2p(i, signers.len() as u16));
    let round3 = rounds.add_round(RoundInput::<MsgRound3>::p2p(i, signers.len() as u16));
    let round4 = rounds.add_round(RoundInput::<MsgRound4>::broadcast(i, signers.len() as u16));
    let mut rounds = rounds.listen(incomings);

    // Round 1
    tracer.round_begins();
    let mut p = State::new(rng, dkls_keyshare.key_share, &derivation_path)
        .map_err(|_e| SignError::FailedCheck("Failed to create state w/ derivation path"))?;

    tracer.stage("Compute round 1 sign msg");
    println!("Compute round 1 sign msg");
    let partial_sign_msg1: SignMsg1 = p.generate_msg1();
    runtime.yield_now().await;
    tracer.stage("Send round 1 sign msg");
    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round1(MsgRound1 {
            msg: partial_sign_msg1.into(),
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    // Round 2
    tracer.round_begins();
    println!("Round 2");
    tracer.receive_msgs();
    let round1_msgs: Vec<SignMsg1> = rounds
        .complete(round1)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me()
        .into_iter()
        .map(|msg| msg.msg.0)
        .collect();
    tracer.msgs_received();

    tracer.stage("Compute round 2 sign msg");
    println!("Compute round 2 sign msg");
    let partial_sign_msg2: Vec<SignMsg2> = p.handle_msg1(rng, round1_msgs)?;
    runtime.yield_now().await;

    tracer.stage("Send round 2 sign msg");
    println!("Send round 2 sign msg");
    tracer.send_msg();
    for msg in partial_sign_msg2.into_iter() {
        outgoings
            .send(Outgoing::p2p(
                msg.to_id as u16,
                Msg::Round2(MsgRound2 {
                    msg: SignMsg2Wrapper(msg),
                }),
            ))
            .await
            .map_err(IoError::send_message)?;
    }
    tracer.msg_sent();

    // Round 3
    tracer.round_begins();

    tracer.receive_msgs();
    let round2_msgs: Vec<SignMsg2> = rounds
        .complete(round2)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me()
        .into_iter()
        .map(|msg| msg.msg.0)
        .collect();
    tracer.msgs_received();

    tracer.stage("Compute round 3 sign msg");
    println!("Compute round 3 sign msg");
    let partial_sign_msg3: Vec<SignMsg3> = p.handle_msg2(rng, round2_msgs)?;
    runtime.yield_now().await;

    tracer.stage("Send round 3 sign msg");
    println!("Send round 3 sign msg");
    tracer.send_msg();
    for msg in partial_sign_msg3.into_iter() {
        outgoings
            .send(Outgoing::p2p(
                msg.to_id as u16,
                Msg::Round3(MsgRound3 {
                    msg: SignMsg3Wrapper(msg),
                }),
            ))
            .await
            .map_err(IoError::send_message)?;
    }
    tracer.msg_sent();

    // Round 4
    tracer.round_begins();

    tracer.receive_msgs();
    let round3_msgs: Vec<SignMsg3> = rounds
        .complete(round3)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me()
        .into_iter()
        .map(|msg| msg.msg.0)
        .collect();
    tracer.msgs_received();

    tracer.stage("Compute round 4 sign msg");
    println!("Compute round 4 sign msg");
    let partial_sign_msg4 = PreSignatureWrapper(p.handle_msg3(round3_msgs)?);
    let partial_signature = create_partial_signature(partial_sign_msg4.clone().0, pre_hashed_msg);
    tracer.stage("Send round 4 pre signature");
    println!("Send round 4 pre signature");
    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round4(MsgRound4 {
            msg: partial_sign_msg4,
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    // Round 5
    tracer.round_begins();

    tracer.receive_msgs();
    let round4_msgs: (Vec<PartialSignature>, Vec<SignMsg4>) = rounds
        .complete(round4)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me()
        .into_iter()
        .map(|msg| create_partial_signature(msg.msg.0, pre_hashed_msg))
        .unzip::<_, _, Vec<_>, Vec<_>>();

    tracer.msgs_received();

    tracer.stage("Compute group signature");
    let group_signature: Signature = combine_signatures(partial_signature.0, round4_msgs.1)?;
    println!("Group signature: {:?}", group_signature);
    Ok(SilentSharedDKLS23EcdsaSignature { group_signature })
}
