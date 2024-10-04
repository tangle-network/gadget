use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, ItemFn, LitInt};

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_tangle_event_handler(
    fn_name_string: &str,
    struct_name: &Ident,
    job_id: &LitInt,
    params_tokens: &[TokenStream],
    result_tokens: &[TokenStream],
    additional_params: &[TokenStream],
    fn_call: &TokenStream,
    event_listener_call: &TokenStream,
) -> TokenStream {
    quote! {
        /// Event handler for the function
        #[doc = "[`"]
        #[doc = #fn_name_string]
        #[doc = "`]"]
        pub struct #struct_name {
            pub service_id: u64,
            pub signer: gadget_sdk::keystore::TanglePairSigner<gadget_sdk::keystore::sp_core_subxt::sr25519::Pair>,
            #(#additional_params)*
        }

        #[automatically_derived]
        #[async_trait::async_trait]
        impl gadget_sdk::events_watcher::substrate::EventHandler<gadget_sdk::clients::tangle::runtime::TangleConfig> for #struct_name {
            fn __init(&self) {
                static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                if !ONCE.load(std::sync::atomic::Ordering::Relaxed) {
                    ONCE.store(true, std::sync::atomic::Ordering::Relaxed);
                    #event_listener_call
                }
            }

            async fn handle(&self, event: &gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled) -> Result<(), gadget_sdk::Error> {
                let mut args_iter = event.args.clone().into_iter();
                #(#params_tokens)*
                #fn_call

                let mut result = Vec::new();
                #(#result_tokens)*

                result
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
