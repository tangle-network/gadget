pub use gadget_client_networking::p2p::P2PClient;

/// `P2pContext` trait provides access to a peer to peer networking client.
pub trait P2pContext {
    fn client(&self) -> P2PClient;
}
