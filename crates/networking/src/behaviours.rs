use crate::key_types::{GossipMsgPublicKey, GossipSignedMsgSignature};
use libp2p::{gossipsub, kad::store::MemoryStore, mdns, request_response, swarm::NetworkBehaviour};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug)]
// TODO: Needs better name
pub enum GossipOrRequestResponse {
    Gossip(GossipMessage),
    Request(MyBehaviourRequest),
    Response(MyBehaviourResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GossipMessage {
    pub topic: String,
    pub raw_payload: Vec<u8>,
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug)]
pub enum MyBehaviourRequest {
    Handshake {
        public_key: GossipMsgPublicKey,
        signature: GossipSignedMsgSignature,
    },
    Message {
        topic: String,
        raw_payload: Vec<u8>,
    },
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug)]
pub enum MyBehaviourResponse {
    Handshaked {
        public_key: GossipMsgPublicKey,
        signature: GossipSignedMsgSignature,
    },
    MessageHandled,
}

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub p2p: request_response::cbor::Behaviour<MyBehaviourRequest, MyBehaviourResponse>,
    pub identify: libp2p::identify::Behaviour,
    pub kadmelia: libp2p::kad::Behaviour<MemoryStore>,
    pub dcutr: libp2p::dcutr::Behaviour,
    pub relay: libp2p::relay::Behaviour,
    pub ping: libp2p::ping::Behaviour,
}
