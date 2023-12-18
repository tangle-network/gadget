use gadget_core::gadget::substrate::Client;
use pallet_jobs_rpc_runtime_api::JobsApi;
use sp_api::ProvideRuntimeApi;
use sp_runtime::{traits::Block, AccountId32};

pub type AccountId = AccountId32;

pub trait ClientWithApi<B: Block>: Client<B> + ProvideRuntimeApi<B> + Clone
where
    <Self as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
}

impl<B, T> ClientWithApi<B> for T
where
    B: Block,
    T: ProvideRuntimeApi<B> + Client<B> + Clone + Send + Sync + 'static,
    <T as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
}
