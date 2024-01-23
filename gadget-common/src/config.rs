pub use crate::client::{AccountId, ClientWithApi};
pub use crate::gadget::network::Network;
pub use crate::gadget::WebbGadgetProtocol;
pub use pallet_jobs_rpc_runtime_api::JobsApi;
pub use sc_client_api::Backend;
pub use sp_api::ProvideRuntimeApi;
pub use sp_runtime::traits::Block;

pub trait ProtocolConfig
where
    <Self::Client as ProvideRuntimeApi<Self::Block>>::Api: JobsApi<Self::Block, AccountId>,
{
    type Network: Network;
    type Block: Block;
    type Backend: Backend<Self::Block>;
    type Protocol: WebbGadgetProtocol<Self::Block, Self::Backend, Self::Client>;
    type Client: ClientWithApi<Self::Block, Self::Backend>;

    fn take_network(&mut self) -> Self::Network;
    fn take_protocol(&mut self) -> Self::Protocol;
    fn take_client(&mut self) -> Self::Client;
}

#[macro_export]
macro_rules! define_protocol {
    ($struct_name:ident) => {
        pub struct $struct_name<N, P, C, B, BE> {
            pub network: Option<N>,
            pub client: Option<C>,
            pub protocol: Option<P>,
            _pd: std::marker::PhantomData<(B, BE)>,
        }

        impl<N, P, C, B, BE> $struct_name<N, P, C, B, BE> {
            pub fn new(network: N, client: C, protocol: P) -> Self {
                Self {
                    network: Some(network),
                    client: Some(client),
                    protocol: Some(protocol),
                    _pd: std::marker::PhantomData,
                }
            }
        }

        impl<
                B: $crate::config::Block,
                BE: $crate::config::Backend<B> + 'static,
                N: $crate::config::Network,
                C: $crate::config::ClientWithApi<B, BE>,
                P: $crate::config::WebbGadgetProtocol<B, BE, C>,
            > $crate::config::ProtocolConfig for $struct_name<N, P, C, B, BE>
        where
            <C as $crate::config::ProvideRuntimeApi<B>>::Api: $crate::config::JobsApi<B, AccountId>,
        {
            type Network = N;
            type Block = B;
            type Backend = BE;
            type Protocol = P;
            type Client = C;

            fn take_network(&mut self) -> Self::Network {
                self.network.take().expect("Network not set")
            }

            fn take_protocol(&mut self) -> Self::Protocol {
                self.protocol.take().expect("Protocol not set")
            }

            fn take_client(&mut self) -> Self::Client {
                self.client.take().expect("Client not set")
            }
        }
    };
}
