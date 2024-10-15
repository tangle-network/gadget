use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, LitInt};

#[allow(clippy::too_many_arguments)]
/// Generates an event handler that initializes all substrate-specific event listeners when called
pub(crate) fn generate_tangle_event_handler(
    fn_name_string: &str,
    struct_name: &Ident,
    job_id: &LitInt,
    params_tokens: &[TokenStream],
    result_tokens: &[TokenStream],
    additional_params: &[TokenStream],
    fn_call: &TokenStream,
    event_listener_calls: &[TokenStream],
    event_type: &TokenStream,
) -> TokenStream {
    let combined_event_listener =
        crate::job::generate_combined_event_listener_selector(struct_name);
    quote! {
        /// Event handler for the function
        #[doc = "[`"]
        #[doc = #fn_name_string]
        #[doc = "`]"]
        #[derive(Clone)]
        pub struct #struct_name {
            pub service_id: u64,
            pub signer: gadget_sdk::keystore::TanglePairSigner<gadget_sdk::keystore::sp_core_subxt::sr25519::Pair>,
            pub client: gadget_sdk::clients::tangle::runtime::TangleClient,
            #(#additional_params)*
        }

        #[automatically_derived]
        #[async_trait::async_trait]
        impl gadget_sdk::events_watcher::substrate::EventHandler<gadget_sdk::clients::tangle::runtime::TangleConfig, #event_type> for #struct_name {
            async fn init(&self) -> Option<gadget_sdk::tokio::sync::oneshot::Receiver<()>> {
                #(#event_listener_calls)*
                #combined_event_listener
            }

            async fn handle(&self, event: &#event_type) -> Result<Vec<gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field<gadget_sdk::subxt_core::utils::AccountId32>>, gadget_sdk::events_watcher::Error> {
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

            fn signer(&self) -> &gadget_sdk::keystore::TanglePairSigner<gadget_sdk::keystore::sp_core_subxt::sr25519::Pair> {
                &self.signer
            }
        }
    }
}
