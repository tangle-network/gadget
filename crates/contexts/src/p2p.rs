pub use gadget_clients::networking::p2p::P2PClient;
pub use gadget_networking::GossipMsgKeyPair;
pub use gadget_std::net::IpAddr;
pub use proc_macro2;

/// `P2pContext` trait provides access to a peer to peer networking client.
pub trait P2pContext {
    fn p2p_client(
        &self,
        name: gadget_std::string::String,
        target_addr: IpAddr,
        target_port: u16,
        my_ecdsa_key: GossipMsgKeyPair,
    ) -> P2PClient;
}
