use ark_serialize::CanonicalDeserialize;
use futures::SinkExt;
use gadget_common::tangle_runtime::*;
use gadget_common::tracer::Tracer;
use ice_frost::ToBytes;
use ice_frost::{
    dkg::{Coefficients, DistributedKeyGeneration, EncryptedSecretShare, Participant, RoundOne},
    keys::DiffieHellmanPrivateKey,
    parameters::ThresholdParameters,
    CipherSuite,
};
use rand_core::{CryptoRng, RngCore};
use round_based::{
    rounds_router::{simple_store::RoundInput, RoundsRouter},
    Delivery, Mpc, MpcParty, Outgoing, ProtocolMessage,
};
use round_based_21 as round_based;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
}

/// Message from round 1
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "ice.frost.keygen.threshold.round1")]
pub struct MsgRound1 {
    pub msg: Vec<u8>,
}
/// Message from round 2
#[derive(Clone, Serialize, Deserialize, udigest::Digestable)]
#[serde(bound = "")]
#[udigest(bound = "")]
#[udigest(tag = "ice.frost.keygen.threshold.round2")]
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
pub struct IceFrostKeyShare {
    pub signing_key: Vec<u8>,
    pub verifying_key: Vec<u8>,
}

pub async fn run_threshold_keygen<C, R, M>(
    mut tracer: Option<&mut dyn Tracer>,
    i: u16,
    t: u16,
    n: u16,
    _role: roles::tss::ThresholdSignatureRoleType,
    rng: &mut R,
    party: M,
) -> Result<IceFrostKeyShare, Error<C>>
where
    R: RngCore + CryptoRng + Clone,
    M: Mpc<ProtocolMessage = Msg>,
    C: CipherSuite,
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
    tracer.stage("Compute round 1 dkg dealer");
    let params = ThresholdParameters::new(n as u32, t as u32)?;
    let (p, p_coeffs, p_dh_sk): (Participant<C>, Coefficients<C>, DiffieHellmanPrivateKey<C>) =
        Participant::<C>::new_dealer(params, 1, rng.clone())?;

    tracer.send_msg();
    tracer.stage("Send proof of knowledge of the DH private key.");
    outgoings
        .send(Outgoing::broadcast(Msg::Round1(MsgRound1 {
            msg: p.to_bytes()?,
        })))
        .await
        .map_err(IoError::send_message)?;
    tracer.msg_sent();

    // Round 2
    tracer.round_begins();

    tracer.receive_msgs();
    tracer.stage("Receive messages and verify nizk PoKs.");
    let participant_results: Vec<Result<Participant<C>, Error<C>>> = rounds
        .complete(round1)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_including_me(MsgRound1 { msg: p.to_bytes()? })
        .into_iter()
        .map(|msg| {
            let p_i = Participant::<C>::deserialize_compressed(msg.msg.as_slice())
                .map_err(|_| Error::SerializationError)?;

            if let Some(key) = p_i.clone().public_key() {
                match p_i.proof_of_dh_private_key.verify(p_i.index, key) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(Error::FrostError(e));
                    }
                };

                if let Some(proof) = p_i.clone().proof_of_secret_key {
                    match proof.verify(p_i.index, key) {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(Error::FrostError(e));
                        }
                    }
                }
            };

            Ok(p_i)
        })
        .collect();

    let mut participants: Vec<Participant<C>> = vec![];
    for participant_result in participant_results {
        participants.push(participant_result?);
    }

    tracer.msgs_received();
    tracer.stage("Bootstrap the DKG");
    let (p_state, _participant_lists) = DistributedKeyGeneration::<RoundOne, C>::bootstrap(
        params,
        &p_dh_sk,
        p.index,
        &p_coeffs,
        &participants,
        rng.clone(),
    )?;
    let p_their_encrypted_secret_shares: &BTreeMap<u32, EncryptedSecretShare<C>> =
        p_state.their_encrypted_secret_shares().unwrap();

    tracer.send_msg();
    tracer.stage("Send each participant their secret encrypted share");
    for (receiver_index, encrypted_secret_share) in p_their_encrypted_secret_shares.iter() {
        outgoings
            .send(Outgoing::p2p(
                *receiver_index as u16,
                Msg::Round2(MsgRound2 {
                    msg: encrypted_secret_share.to_bytes()?,
                }),
            ))
            .await
            .map_err(IoError::send_message)?;
    }
    tracer.msg_sent();

    // Round 3
    tracer.round_begins();

    tracer.receive_msgs();
    tracer.stage("Receive encrypted secret shares from other participants.");
    let my_encrypted_secret_share = p_their_encrypted_secret_shares.get(&p.index).unwrap();
    let encryption_results: Vec<Result<EncryptedSecretShare<C>, Error<C>>> = rounds
        .complete(round2)
        .await
        .map_err(IoError::receive_message)?
        .into_vec_including_me(MsgRound2 {
            msg: my_encrypted_secret_share.to_bytes()?,
        })
        .into_iter()
        .map(|msg| {
            EncryptedSecretShare::<C>::deserialize_compressed(msg.msg.as_slice())
                .map_err(|_| Error::SerializationError)
        })
        .collect();

    let mut my_encrypted_secret_shares = Vec::new();
    for encryption_result in encryption_results.into_iter() {
        my_encrypted_secret_shares.push(encryption_result?);
    }

    tracer.msgs_received();

    tracer.stage("Compute round 3 dkg state");
    let (p_state, complaints) = p_state
        .to_round_two(&my_encrypted_secret_shares, rng)
        .unwrap();

    tracer.stage("Ensure no complaints");
    if !complaints.is_empty() {
        return Err(Error::FrostError(ice_frost::Error::Complaint(complaints)));
    }

    tracer.stage("Compute key share");
    let (group_key, p1_sk) = p_state.finish()?;

    tracer.protocol_ends();
    Ok(IceFrostKeyShare {
        signing_key: p1_sk.to_bytes()?,
        verifying_key: group_key.to_bytes()?,
    })
}
