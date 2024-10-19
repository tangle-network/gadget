use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, LitInt};

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_tangle_event_handler(
    struct_name: &Ident,
    _job_id: &LitInt,
    _params_tokens: &[TokenStream],
    _result_tokens: &[TokenStream],
    _fn_call: &TokenStream,
) -> TokenStream {
    /*
           #[automatically_derived]
       #[async_trait::async_trait]
       impl gadget_sdk::events_watcher::substrate::EventHandler<gadget_sdk::clients::tangle::runtime::TangleConfig, gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled> for #struct_name {
           async fn handle(&self, event: &gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled) -> Result<Vec<gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field<gadget_sdk::subxt_core::utils::AccountId32>>, gadget_sdk::events_watcher::Error> {
               use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field;
               let mut args_iter = event.args.clone().into_iter();
               #(#params_tokens)*
               #fn_call

               let mut result = Vec::new();
               #(#result_tokens)*

               Ok(result)
           }

           /// Returns the job ID
           fn job_id(&self) -> u8 {
               #job_id
           }

           /// Returns the service ID
           fn service_id(&self) -> u64 {
               self.service_id
           }

           fn signer(&self) -> &gadget_sdk::keystore::TanglePairSigner<gadget_sdk::ext::sp_core::sr25519::Pair> {
               &self.signer
           }
       }
    */
    quote! {
        impl gadget_sdk::event_listener::markers::IsTangle for #struct_name {}
    }
}
