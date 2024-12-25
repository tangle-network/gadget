pub use gadget_clients::networking::p2p::P2PClient;

/// `P2pContext` trait provides access to a peer to peer networking client.
pub trait P2pContext {
    fn p2p_client(&self) -> P2PClient;
}
