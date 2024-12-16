/// `P2PContext` trait provides access to a peer to peer networking client.
pub trait P2PContext {
    fn client(&self) -> gadget_client_networking::p2p::P2PClient;
}
