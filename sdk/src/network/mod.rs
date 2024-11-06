use core::fmt::Display;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sp_core::ecdsa;

use crate::error::Error;

use self::channels::UserID;

pub mod channels;
pub mod gossip;
pub mod handlers;
#[cfg(target_family = "wasm")]
pub mod matchbox;
pub mod messaging;
pub mod setup;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct IdentifierInfo {
    pub block_id: Option<u64>,
    pub session_id: Option<u64>,
    pub retry_id: Option<u64>,
    pub task_id: Option<u64>,
}

impl Display for IdentifierInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let block_id = self
            .block_id
            .map(|id| format!("block_id: {}", id))
            .unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| format!("session_id: {}", id))
            .unwrap_or_default();
        let retry_id = self
            .retry_id
            .map(|id| format!("retry_id: {}", id))
            .unwrap_or_default();
        let task_id = self
            .task_id
            .map(|id| format!("task_id: {}", id))
            .unwrap_or_default();
        write!(f, "{} {} {} {}", block_id, session_id, retry_id, task_id)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ParticipantInfo {
    pub user_id: u16,
    pub ecdsa_key: Option<sp_core::ecdsa::Public>,
}

impl Display for ParticipantInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ecdsa_key = self
            .ecdsa_key
            .map(|key| format!("ecdsa_key: {}", key))
            .unwrap_or_default();
        write!(f, "user_id: {}, {}", self.user_id, ecdsa_key)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolMessage {
    pub identifier_info: IdentifierInfo,
    pub sender: ParticipantInfo,
    pub recipient: Option<ParticipantInfo>,
    pub payload: Vec<u8>,
}

impl Display for ProtocolMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "identifier_info: {}, sender: {}, recipient: {:?}, payload: {:?}",
            self.identifier_info, self.sender, self.recipient, self.payload
        )
    }
}

#[async_trait]
pub trait Network: Send + Sync + Clone + 'static {
    async fn next_message(&self) -> Option<ProtocolMessage>;
    async fn send_message(&self, message: ProtocolMessage) -> Result<(), Error>;

    /// If the network implementation requires a custom runtime, this function
    /// should be manually implemented to keep the network alive
    async fn run(self) -> Result<(), Error> {
        Ok(())
    }

    fn build_protocol_message<Payload: Serialize>(
        identifier_info: IdentifierInfo,
        from: UserID,
        to: Option<UserID>,
        payload: &Payload,
        from_account_id: Option<ecdsa::Public>,
        to_network_id: Option<ecdsa::Public>,
    ) -> ProtocolMessage {
        let sender_participant_info = ParticipantInfo {
            user_id: from,
            ecdsa_key: from_account_id,
        };
        let receiver_participant_info = to.map(|to| ParticipantInfo {
            user_id: to,
            ecdsa_key: to_network_id,
        });
        ProtocolMessage {
            identifier_info,
            sender: sender_participant_info,
            recipient: receiver_participant_info,
            payload: serialize(payload).expect("Failed to serialize message"),
        }
    }
}

pub fn deserialize<'a, T>(data: &'a [u8]) -> Result<T, serde_json::Error>
where
    T: Deserialize<'a>,
{
    serde_json::from_slice::<T>(data)
}

pub fn serialize(object: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(object)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use futures::{stream, StreamExt};
    use gossip::GossipHandle;
    use serde::{Deserialize, Serialize};
    use sp_core::Pair;

    const TOPIC: &str = "/gadget/test/1.0.0";

    #[derive(Debug, Serialize, Deserialize, Clone)]
    enum Msg {
        Round1(Round1Msg),
        Round2(Round2Msg),
        Round3(Round3Msg),
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct Round1Msg {
        pub power: u16,
        pub hitpoints: u16,
        pub armor: u16,
        pub name: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct Round2Msg {
        pub x: u16,
        pub y: u16,
        pub z: u16,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct Round3Msg {
        rotation: u16,
        velocity: (u16, u16, u16),
    }

    // NOTE: if you lower the number of nodes to 2, this test passes without issues.
    const NODE_COUNT: u16 = 3;

    pub fn setup_log() {
        use tracing_subscriber::util::SubscriberInitExt;
        let env_filter = tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("tokio=off".parse().unwrap())
            .add_directive("hyper=off".parse().unwrap())
            .add_directive("gadget=debug".parse().unwrap());

        let _ = tracing_subscriber::fmt::SubscriberBuilder::default()
            .compact()
            .without_time()
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
            .with_target(false)
            .with_env_filter(env_filter)
            .with_test_writer()
            .finish()
            .try_init();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn p2p() {
        setup_log();
        let nodes = stream::iter(0..NODE_COUNT)
            .map(|_| node())
            .collect::<Vec<_>>()
            .await;
        // wait for the nodes to connect to each other
        let max_retries = 30 * NODE_COUNT;
        let mut retry = 0;
        loop {
            crate::debug!(%NODE_COUNT, %max_retries, %retry, "Checking if all nodes are connected to each other");
            let connected = nodes
                .iter()
                .map(|node| node.connected_peers())
                .collect::<Vec<_>>();

            let all_connected = connected
                .iter()
                .enumerate()
                .inspect(|(node, peers)| crate::debug!(%node, %peers, "Connected peers"))
                .all(|(_, &peers)| peers == usize::from(NODE_COUNT) - 1);
            if all_connected {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            retry += 1;
            if retry > max_retries {
                panic!("Failed to connect all nodes to each other");
            }
        }
        crate::debug!("All nodes are connected to each other");
        let mut tasks = Vec::new();
        for (i, node) in nodes.into_iter().enumerate() {
            let task = tokio::spawn(run_protocol(node, i as u16));
            tasks.push(task);
        }
        // Wait for all tasks to finish
        let results = futures::future::try_join_all(tasks)
            .await
            .expect("Failed to run protocol");
        // Assert that all are okay.
        assert!(
            results.iter().all(|r| r.is_ok()),
            "Some nodes failed to run protocol"
        );
    }

    async fn run_protocol<N: Network>(node: N, i: u16) -> Result<(), crate::Error> {
        let round1_span = tracing::debug_span!(target: "gadget", "Round 1", node = %i);
        let round1_guard = round1_span.enter();
        // Round 1 (broadcast)
        let msg = {
            let round = Round1Msg {
                power: i * 100,
                hitpoints: (i + 1) * 50,
                armor: i + 2,
                name: format!("Player {}", i),
            };

            GossipHandle::build_protocol_message(
                IdentifierInfo {
                    block_id: None,
                    session_id: None,
                    retry_id: None,
                    task_id: None,
                },
                i,
                None,
                &Msg::Round1(round),
                None,
                None,
            )
        };

        crate::debug!("Broadcast Message");
        node.send_message(msg).await?;

        // Wait for all other nodes to send their messages
        let mut msgs = BTreeMap::new();
        while let Some(msg) = node.next_message().await {
            let m = deserialize::<Msg>(&msg.payload).unwrap();
            crate::debug!(from = %msg.sender.user_id, ?m, "Received message");
            // Expecting Round1 message
            assert!(
                matches!(m, Msg::Round1(_)),
                "Expected Round1 message but got {:?} from node {}",
                m,
                msg.sender.user_id,
            );
            let old = msgs.insert(msg.sender.user_id, m);
            assert!(
                old.is_none(),
                "Duplicate message from node {}",
                msg.sender.user_id
            );
            // Break if all messages are received
            if msgs.len() == usize::from(NODE_COUNT) - 1 {
                break;
            }
        }
        crate::debug!("Done");
        drop(round1_guard);
        drop(round1_span);

        let round2_span = tracing::debug_span!(target: "gadget", "Round 2", node = %i);
        let round2_guard = round2_span.enter();
        // Round 2 (P2P)
        let msg = Round2Msg {
            x: i * 10,
            y: (i + 1) * 20,
            z: i + 2,
        };
        let msgs = (0..NODE_COUNT)
            .filter(|&j| j != i)
            .map(|j| {
                GossipHandle::build_protocol_message(
                    IdentifierInfo {
                        block_id: None,
                        session_id: None,
                        retry_id: None,
                        task_id: None,
                    },
                    i,
                    Some(j),
                    &Msg::Round2(msg.clone()),
                    None,
                    None,
                )
            })
            .collect::<Vec<_>>();
        for msg in msgs {
            let to = msg.recipient.map(|r| r.user_id).expect(
                "Recipient should be present for P2P message. This is a bug in the test code",
            );
            crate::debug!(%to, "Send P2P Message");
            node.send_message(msg).await?;
        }

        // Wait for all other nodes to send their messages
        let mut msgs = BTreeMap::new();
        while let Some(msg) = node.next_message().await {
            let m = deserialize::<Msg>(&msg.payload).unwrap();
            crate::debug!(from = %msg.sender.user_id, ?m, "Received message");
            // Expecting Round2 message
            assert!(
                matches!(m, Msg::Round2(_)),
                "Expected Round2 message but got {:?} from node {}",
                m,
                msg.sender.user_id,
            );
            let old = msgs.insert(msg.sender.user_id, m);
            assert!(
                old.is_none(),
                "Duplicate message from node {}",
                msg.sender.user_id
            );
            // Break if all messages are received
            if msgs.len() == usize::from(NODE_COUNT) - 1 {
                break;
            }
        }
        crate::debug!("Done");
        drop(round2_guard);
        drop(round2_span);

        // Round 3 (broadcast)
        let round3_span = tracing::debug_span!(target: "gadget", "Round 3", node = %i);
        let round3_guard = round3_span.enter();

        let msg = {
            let round = Round3Msg {
                rotation: i * 30,
                velocity: (i + 1, i + 2, i + 3),
            };
            GossipHandle::build_protocol_message(
                IdentifierInfo {
                    block_id: None,
                    session_id: None,
                    retry_id: None,
                    task_id: None,
                },
                i,
                None,
                &Msg::Round3(round),
                None,
                None,
            )
        };

        crate::debug!("Broadcast Message");
        node.send_message(msg).await?;

        // Wait for all other nodes to send their messages
        let mut msgs = BTreeMap::new();
        while let Some(msg) = node.next_message().await {
            let m = deserialize::<Msg>(&msg.payload).unwrap();
            crate::debug!(from = %msg.sender.user_id, ?m, "Received message");
            // Expecting Round3 message
            assert!(
                matches!(m, Msg::Round3(_)),
                "Expected Round3 message but got {:?} from node {}",
                m,
                msg.sender.user_id,
            );
            let old = msgs.insert(msg.sender.user_id, m);
            assert!(
                old.is_none(),
                "Duplicate message from node {}",
                msg.sender.user_id
            );
            // Break if all messages are received
            if msgs.len() == usize::from(NODE_COUNT) - 1 {
                break;
            }
        }
        crate::debug!("Done");
        drop(round3_guard);
        drop(round3_span);

        crate::info!(node = i, "Protocol completed");

        Ok(())
    }

    fn node() -> gossip::GossipHandle {
        let identity = libp2p::identity::Keypair::generate_ed25519();
        let ecdsa_key = sp_core::ecdsa::Pair::generate().0;
        let bind_port = 0;
        let bind_ip = [0, 0, 0, 0].into();
        setup::start_p2p_network(setup::NetworkConfig::new_service_network(
            identity,
            ecdsa_key,
            Default::default(),
            bind_ip,
            bind_port,
            TOPIC,
        ))
        .unwrap()
    }
}
