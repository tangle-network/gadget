use crate::NetworkEvent;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Gossip error: {0}")]
    GossipError(String),

    #[error("Messaging error: {0}")]
    MessagingError(String),

    #[error("Round based error: {0}")]
    RoundBasedError(String),

    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Attempted to start a network with too many topics: {0}")]
    TooManyTopics(usize),
    #[error("All topics must be unique")]
    DuplicateTopics,

    #[error("No network found")]
    NoNetworkFound,

    #[error("Kademlia is not activated")]
    KademliaNotActivated,

    #[error("Other error: {0}")]
    Other(String),

    // std compat
    #[error(transparent)]
    Io(#[from] std::io::Error),

    // libp2p compat
    #[error(transparent)]
    InvalidProtocol(#[from] libp2p::swarm::InvalidProtocol),

    #[error(transparent)]
    NoKnownPeers(#[from] libp2p::kad::NoKnownPeers),

    #[error(transparent)]
    Dial(#[from] libp2p::swarm::DialError),

    #[error(transparent)]
    Noise(#[from] libp2p::noise::Error),

    #[error(transparent)]
    Behaviour(#[from] libp2p::BehaviourBuilderError),

    #[error(transparent)]
    Subscription(#[from] libp2p::gossipsub::SubscriptionError),

    #[error(transparent)]
    TransportIo(#[from] libp2p::TransportError<std::io::Error>),

    #[error(transparent)]
    Multiaddr(#[from] libp2p::multiaddr::Error),

    #[error(transparent)]
    TokioSendError(#[from] tokio::sync::mpsc::error::SendError<NetworkEvent>),

    #[error(transparent)]
    CrossbeamSendError(#[from] crossbeam_channel::SendError<NetworkEvent>),
}
