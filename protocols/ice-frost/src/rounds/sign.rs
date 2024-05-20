use futures::SinkExt;
use gadget_common::tangle_runtime::*;
use gadget_common::tracer::Tracer;
use ice_frost::keys::{GroupVerifyingKey, IndividualSigningKey};
use ice_frost::parameters::ThresholdParameters;
use ice_frost::sign::{
    generate_commitment_share_lists, PartialThresholdSignature, PublicCommitmentShareList,
    SignatureAggregator,
};
use ice_frost::CipherSuite;
use ice_frost::{FromBytes, ToBytes};
use rand_core::{CryptoRng, RngCore};
use round_based::rounds_router::simple_store::RoundInput;
use round_based::rounds_router::RoundsRouter;

use round_based::ProtocolMessage;
use round_based::{Delivery, Mpc, MpcParty, Outgoing};
use round_based_21 as round_based;
use serde::{Deserialize, Serialize};

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
    n: u16,
    signers: Vec<u16>,
    key_share: (IndividualSigningKey<C>, GroupVerifyingKey<C>),
    message_to_sign: &[u8],
    _role: roles::tss::ThresholdSignatureRoleType,
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
    let MpcParty { delivery, .. } = party.into_party();
    let (incomings, mut outgoings) = delivery.split();
    let mut rounds = RoundsRouter::<Msg>::builder();
    let round1 = rounds.add_round(RoundInput::<MsgRound1>::broadcast(i, signers.len() as u16));
    let round2 = rounds.add_round(RoundInput::<MsgRound2>::broadcast(i, signers.len() as u16));
    let mut rounds = rounds.listen(incomings);

    tracer.protocol_begins();

    // Round 1
    tracer.stage("Round 1");
    let params = ThresholdParameters::new(n as u32, signers.len() as u32)?;
    let (p_sk, group_key) = key_share;
    let mut aggregator = SignatureAggregator::new(params, group_key, message_to_sign);
    let (p_public_comshares, mut p_secret_comshares) =
        generate_commitment_share_lists(rng, &p_sk, 1)?;

    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round1(MsgRound1 {
            msg: p_public_comshares.to_bytes()?,
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    // Round 2
    tracer.stage("Round 2");
    tracer.receive_msgs();
    let received_comshares_results = rounds
        .complete(round1)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_including_me(MsgRound1 {
            msg: p_public_comshares.to_bytes()?,
        })
        .into_iter()
        .map(|msg| PublicCommitmentShareList::<C>::from_bytes(&msg.msg).map_err(Error::from))
        .collect::<Vec<Result<PublicCommitmentShareList<C>, Error<C>>>>();

    let mut received_comshares = Vec::<PublicCommitmentShareList<C>>::new();
    for received_comshare_result in received_comshares_results {
        received_comshares.push(received_comshare_result?);
    }
    tracer.msgs_received();

    tracer.stage("Process received comshares");
    for (index, received_comshare) in received_comshares.iter().enumerate() {
        if index + 1 == i as usize {
            continue;
        }

        aggregator.include_signer(
            (index + 1) as u32,
            received_comshare.commitments[0],
            &p_sk.to_public(),
        )?;
    }

    tracer.stage("Compute partial signature");
    let signers = aggregator.signers();
    let partial_sig = p_sk
        .sign(
            message_to_sign,
            &group_key,
            &mut p_secret_comshares,
            0,
            signers,
        )
        .unwrap();

    tracer.send_msg();
    outgoings
        .send(Outgoing::broadcast(Msg::Round2(MsgRound2 {
            msg: partial_sig.to_bytes()?,
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    // Round 3
    tracer.stage("Round 3");
    tracer.receive_msgs();
    let received_partial_sigs = rounds
        .complete(round2)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_without_me();
    tracer.msgs_received();

    tracer.stage("Aggregate partial signatures");
    aggregator.include_partial_signature(&partial_sig);
    for received_partial_sig in received_partial_sigs {
        let p_sig = PartialThresholdSignature::from_bytes(&received_partial_sig.msg).unwrap();
        aggregator.include_partial_signature(&p_sig);
    }

    let aggregator = aggregator.finalize()?;
    let _threshold_signature = aggregator.aggregate()?;

    Ok(FrostSignature {
        group_signature: vec![],
    })
}
