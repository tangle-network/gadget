use crate::client::{AccountId, ClientWithApi};
use crate::gadget::network::Network;
use crate::gadget::WebbGadgetProtocol;
use pallet_jobs_rpc_runtime_api::JobsApi;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;

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
