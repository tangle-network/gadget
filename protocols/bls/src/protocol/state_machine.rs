use crate::protocol::state_machine::payloads::{Round1Payload, RoundPayload};
use gadget_common::config::DebugLogger;
use gennaro_dkg::{
    Parameters, Participant, Round1BroadcastData, Round1P2PData, Round2EchoBroadcastData,
    Round3BroadcastData, Round4EchoBroadcastData, SecretParticipantImpl,
};
use round_based::{IsCritical, Msg, StateMachine};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::num::NonZeroUsize;
use std::time::Duration;

pub struct BlsStateMachine {
    pub me: Participant<SecretParticipantImpl<Group>, Group>,
    messages: Vec<Msg<RoundPayload>>,
    party_index: u16,
    n: u16,
    state: BlsState,
    #[allow(dead_code)]
    logger: DebugLogger,
}

#[derive(Debug)]
pub struct BlsStateMachineError {
    #[allow(dead_code)]
    reason: String,
}

impl Display for BlsStateMachineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

pub type Group = bls12_381_plus::G1Projective;
pub type GroupBlsful = blsful::Bls12381G1Impl;

impl BlsStateMachine {
    pub fn new(i: u16, t: u16, n: u16, logger: DebugLogger) -> Result<Self, BlsStateMachineError> {
        let i = NonZeroUsize::new(i as usize).expect("I > 0");
        let n = NonZeroUsize::new(n as usize).expect("N > 0");
        let t = NonZeroUsize::new(t as usize).expect("T > 0");
        let parameters = Parameters::new(t, n);
        let me = Participant::new(i, parameters).map_err(|e| BlsStateMachineError {
            reason: e.to_string(),
        })?;

        Ok(BlsStateMachine {
            me,
            messages: vec![],
            party_index: i.get() as u16,
            n: n.get() as u16,
            state: BlsState {
                round1_broadcasts: BTreeMap::new(),
                round1_p2p: BTreeMap::new(),
                round2_broadcasts: BTreeMap::new(),
                round3_broadcasts: BTreeMap::new(),
                round4_broadcasts: BTreeMap::new(),
                current_round: 0,
            },
            logger,
        })
    }
}

struct BlsState {
    round1_broadcasts: BTreeMap<usize, Round1BroadcastData<Group>>,
    round1_p2p: BTreeMap<usize, Round1P2PData>,
    round2_broadcasts: BTreeMap<usize, Round2EchoBroadcastData>,
    round3_broadcasts: BTreeMap<usize, Round3BroadcastData<Group>>,
    round4_broadcasts: BTreeMap<usize, Round4EchoBroadcastData<Group>>,
    current_round: usize,
}

impl BlsState {
    // If the count of any of the maps is equal to n - 1, we are ready to proceed
    fn ready_to_proceed(&self, n: usize) -> bool {
        let count_needed = n - 1;

        if self.current_round == 0 {
            true
        } else if self.current_round == 1 {
            self.round1_broadcasts.len() == count_needed && self.round1_p2p.len() == count_needed
        } else if self.current_round == 2 {
            self.round2_broadcasts.len() == count_needed
        } else if self.current_round == 3 {
            self.round3_broadcasts.len() == count_needed
        } else if self.current_round == 4 {
            self.round4_broadcasts.len() == count_needed
        } else {
            false
        }
    }

    fn store_incoming_message(&mut self, sender: usize, msg: RoundPayload) {
        match msg {
            RoundPayload::Round1(payload) => match payload {
                Round1Payload::P2P(p2p) => {
                    self.round1_p2p.insert(sender, p2p);
                }
                Round1Payload::Broadcast(broadcast) => {
                    self.round1_broadcasts.insert(sender, broadcast);
                }
            },
            RoundPayload::Round2(payload) => {
                self.round2_broadcasts.insert(sender, payload);
            }
            RoundPayload::Round3(payload) => {
                self.round3_broadcasts.insert(sender, payload);
            }
            RoundPayload::Round4(payload) => {
                self.round4_broadcasts.insert(sender, payload);
            }
            _ => {}
        }
    }
}

impl StateMachine for BlsStateMachine {
    type MessageBody = RoundPayload;
    type Err = BlsStateMachineError;
    type Output = Participant<SecretParticipantImpl<Group>, Group>;

    fn handle_incoming(&mut self, msg: Msg<Self::MessageBody>) -> Result<(), Self::Err> {
        let (sender, _receiver, payload) = (msg.sender, msg.receiver, msg.body);
        self.state.store_incoming_message(sender as usize, payload);
        Ok(())
    }

    fn message_queue(&mut self) -> &mut Vec<Msg<Self::MessageBody>> {
        &mut self.messages
    }

    fn wants_to_proceed(&self) -> bool {
        self.state.ready_to_proceed(self.n as usize)
    }

    fn proceed(&mut self) -> Result<(), Self::Err> {
        let current_round = self.current_round();

        if current_round == 0 {
            let (broadcast, p2p) = self.me.round1().map_err(|e| BlsStateMachineError {
                reason: e.to_string(),
            })?;

            let broadcast_msg = Msg {
                sender: self.party_index,
                receiver: None,
                body: RoundPayload::Round1(Round1Payload::Broadcast(broadcast)),
            };

            self.messages.push(broadcast_msg);

            for (receiver, round_1_msg) in p2p {
                let p2p_msg = Msg {
                    sender: self.party_index,
                    receiver: Some(receiver as u16),
                    body: RoundPayload::Round1(Round1Payload::P2P(round_1_msg)),
                };

                self.messages.push(p2p_msg);
            }

            self.state.current_round += 1;
        } else if current_round == 1 {
            let round2_messages = self
                .me
                .round2(
                    self.state.round1_broadcasts.clone(),
                    self.state.round1_p2p.clone(),
                )
                .map_err(|e| BlsStateMachineError {
                    reason: e.to_string(),
                })?;

            let msg = Msg {
                sender: self.party_index,
                receiver: None,
                body: RoundPayload::Round2(round2_messages),
            };

            self.messages.push(msg);
            self.state.current_round += 1;
        } else if current_round == 2 {
            let round3_messages = self.me.round3(&self.state.round2_broadcasts).map_err(|e| {
                BlsStateMachineError {
                    reason: e.to_string(),
                }
            })?;

            let msg = Msg {
                sender: self.party_index,
                receiver: None,
                body: RoundPayload::Round3(round3_messages),
            };

            self.messages.push(msg);
            self.state.current_round += 1;
        } else if current_round == 3 {
            let round4_messages = self.me.round4(&self.state.round3_broadcasts).map_err(|e| {
                BlsStateMachineError {
                    reason: e.to_string(),
                }
            })?;

            let msg = Msg {
                sender: self.party_index,
                receiver: None,
                body: RoundPayload::Round4(round4_messages),
            };

            self.messages.push(msg);
            self.state.current_round += 1;
        } else if current_round == 4 {
            self.me
                .round5(&self.state.round4_broadcasts)
                .map_err(|e| BlsStateMachineError {
                    reason: e.to_string(),
                })?;
            self.state.current_round += 1;
            // Done
        }

        Ok(())
    }

    fn round_timeout(&self) -> Option<Duration> {
        None
    }

    fn round_timeout_reached(&mut self) -> Self::Err {
        BlsStateMachineError {
            reason: "Bls Timeout reached".to_string(),
        }
    }

    fn is_finished(&self) -> bool {
        self.current_round() == 5
    }

    fn pick_output(&mut self) -> Option<Result<Self::Output, Self::Err>> {
        if self.is_finished() {
            Some(Ok(self.me.clone()))
        } else {
            None
        }
    }

    fn current_round(&self) -> u16 {
        self.state.current_round as u16
    }

    fn total_rounds(&self) -> Option<u16> {
        Some(5)
    }

    fn party_ind(&self) -> u16 {
        self.party_index
    }

    fn parties(&self) -> u16 {
        self.n
    }
}

impl IsCritical for BlsStateMachineError {
    fn is_critical(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::state_machine::{BlsStateMachine, GroupBlsful};
    use blsful::{PublicKey, Signature, SignatureSchemes};
    use gadget_common::config::DebugLogger;
    use round_based::dev::AsyncSimulation;

    #[tokio::test]
    async fn test_bls_state_machine() {
        const T: u16 = 2;
        const N: u16 = 3;
        const TEST_MSG: &[u8] = b"Hello, world";

        let simulation = AsyncSimulation::<BlsStateMachine>::new()
            .add_party(
                BlsStateMachine::new(
                    1,
                    T,
                    N,
                    DebugLogger {
                        peer_id: "1".to_string(),
                    },
                )
                .unwrap(),
            )
            .add_party(
                BlsStateMachine::new(
                    2,
                    T,
                    N,
                    DebugLogger {
                        peer_id: "2".to_string(),
                    },
                )
                .unwrap(),
            )
            .add_party(
                BlsStateMachine::new(
                    3,
                    T,
                    N,
                    DebugLogger {
                        peer_id: "3".to_string(),
                    },
                )
                .unwrap(),
            )
            .run()
            .await;

        for res in simulation {
            assert!(res.is_ok());
            if let Ok(participant) = res {
                let _pk = participant
                    .get_public_key()
                    .expect("Public key should be some");
                let sk = participant
                    .get_secret_share()
                    .expect("Secret should be some");

                let sk =
                    blsful::SecretKey::<GroupBlsful>::from_be_bytes(&sk.to_be_bytes()).unwrap();
                let shares = sk
                    .split_with_rng(T as _, N as _, &mut rand::thread_rng())
                    .unwrap();

                let sig1 = shares[0].sign(SignatureSchemes::Basic, TEST_MSG).unwrap();
                let sig2 = shares[1].sign(SignatureSchemes::Basic, TEST_MSG).unwrap();
                let sig3 = shares[2].sign(SignatureSchemes::Basic, TEST_MSG).unwrap();

                let pks1 = shares[0].public_key().unwrap();
                let pks2 = shares[1].public_key().unwrap();
                let pks3 = shares[2].public_key().unwrap();

                assert!(sig1.verify(&pks1, TEST_MSG).is_ok());
                assert!(sig2.verify(&pks2, TEST_MSG).is_ok());
                assert!(sig3.verify(&pks3, TEST_MSG).is_ok());

                let combined_signature = Signature::from_shares(&[sig1, sig2, sig3]).unwrap();
                let combined_pk = PublicKey::from_shares(&[pks1, pks2, pks3]).unwrap();
                assert!(combined_signature.verify(&combined_pk, TEST_MSG).is_ok());
            }
        }
    }
}

pub(crate) mod payloads {
    use crate::protocol::state_machine::Group;
    use gennaro_dkg::{
        Round1BroadcastData, Round1P2PData, Round2EchoBroadcastData, Round3BroadcastData,
        Round4EchoBroadcastData,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone)]
    pub enum RoundPayload {
        Round1(Round1Payload),
        Round2(Round2EchoBroadcastData),
        Round3(Round3BroadcastData<Group>),
        Round4(Round4EchoBroadcastData<Group>),
        PublicKeyGossip(Vec<u8>),
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub enum Round1Payload {
        Broadcast(Round1BroadcastData<Group>),
        P2P(Round1P2PData),
    }
}
