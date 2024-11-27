/// `GossipNetworkContext` trait provides access to the network client from the context.
pub trait GossipNetworkContext {
    /// Get the Goossip client from the context.
    fn gossip_network(&self) -> &crate::network::gossip::GossipHandle;
}
