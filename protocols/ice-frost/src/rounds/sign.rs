use futures::SinkExt;
use gadget_common::tangle_runtime::*;
use gadget_common::tracer::Tracer;
use ice_frost::keys::{DiffieHellmanPrivateKey, GroupVerifyingKey};
use ice_frost::parameters::ThresholdParameters;
use ice_frost::sign::generate_commitment_share_lists;
use ice_frost::CipherSuite;
use rand_core::{CryptoRng, RngCore};
use round_based::rounds_router::simple_store::RoundInput;
use round_based::rounds_router::RoundsRouter;
use round_based::runtime::AsyncRuntime;
use round_based::ProtocolMessage;
use round_based::{Delivery, Mpc, MpcParty, Outgoing};
use round_based_21 as round_based;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{Error, IoError};

/// Message of threshold FROST signing protocol
#[derive(ProtocolMessage, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub enum Msg {
    /// Round 1 message
    Round1(MsgRound1),
    /// Round 2 message
    Round2(MsgRound2),
}

/// Message from round 1
#[derive(Clone, Debug, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "ice.frost.sign.threshold.round1")]
pub struct MsgRound1 {
    pub msg: Vec<u8>,
}
/// Message from round 2
#[derive(Clone, Debug, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "ice.frost.sign.threshold.round2")]
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
    frost_keyshare: (DiffieHellmanPrivateKey<C>, GroupVerifyingKey<C>),
    message_to_sign: &[u8],
    role: roles::tss::ThresholdSignatureRoleType,
    rng: &mut R,
    party: M,
) -> Result<FrostSignature, Error<C>>
where
    R: RngCore + CryptoRng,
    M: Mpc<ProtocolMessage = Msg>,
    C: CipherSuite,
{
    tracer.protocol_begins();

    tracer.stage("Setup networking");
    let MpcParty {
        delivery, runtime, ..
    } = party.into_party();
    let (incomings, mut outgoings) = delivery.split();
    let mut rounds = RoundsRouter::<Msg>::builder();
    let round1 = rounds.add_round(RoundInput::<MsgRound1>::broadcast(i, signers.len() as u16));
    let round2 = rounds.add_round(RoundInput::<MsgRound2>::broadcast(i, signers.len() as u16));
    let mut rounds = rounds.listen(incomings);

    tracer.protocol_begins();

    // Round 1
    tracer.stage("Round 1");
    let params = ThresholdParameters::new(n as u32, t as u32);
    let mut aggregator = SignatureAggregator::new(params, group_key, &message_to_sign[..]);
    let (p_sk, group_key) = frost_keyshare;
    let (p_public_comshares, mut p_secret_comshares) =
        generate_commitment_share_lists(&mut OsRng, &p_sk, 1)?;

    tracer.send_msg();
    outgoings
        .send(
            (
                round1,
                Msg::Round1(MsgRound1 {
                    msg: p_public_comshares.to_bytes(),
                }),
            )
                .into(),
        )
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    // Round 2
    tracer.stage("Round 2");
    tracer.receive_msg();
    let received_comshares = rounds
        .complete(round1)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_including_me(MsgRound1 {
            msg: p_public_comshares.to_bytes(),
        })
        .collect::<Vec<_>>();
    tracer.msg_received();

    tracer.stage("Compute partial signature");
    let message_hash = C::h4(&message_to_sign[..]);
    let partial_sig = p_sk
        .sign(
            &message_hash,
            &group_key,
            &mut received_comshares,
            0,
            signers,
        )
        .unwrap();

    tracer.send_msg();
    outgoings
        .send(
            (
                round2,
                Msg::Round2(MsgRound2 {
                    msg: partial_sig.to_bytes(),
                }),
            )
                .into(),
        )
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    // Round 3
    tracer.stage("Round 3");
    tracer.receive_msg();
    let received_partial_sigs = rounds
        .complete(round2)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me()
        .collect::<Vec<_>>();
    tracer.msg_received();

    tracer.stage("Aggregate partial signatures");
    aggregator.include_partial_signature(&partial_sig);
    for received_partial_sig in received_partial_sigs {
        let p_sig = PartialSignature::from_bytes(&received_partial_sig.msg).unwrap();
        aggregator.include_partial_signature(p_sig);
    }

    let aggregator = aggregator.finalize()?;
    let threshold_signature = aggregator.aggregate()?;

    Ok(FrostSignature {
        group_signature: vec![],
    })
}
