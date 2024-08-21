use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::job::EventHandlerArgs;

pub(crate) fn generate_eigenlayer_event_handler(
    fn_name_string: &str,
    struct_name: &Ident,
    event_handler: &EventHandlerArgs,
    additional_params: &[TokenStream],
) -> TokenStream {
    let instance_base = event_handler.instance().unwrap();
    let instance_name = format_ident!("{}Instance", instance_base);
    let instance = quote! { #instance_base::#instance_name<T::T, T::P, T::N> };
    let event = event_handler.event().unwrap();
    let _callback = event_handler.callback().unwrap();

    quote! {
        use alloy_network::Network;
        use alloy_provider::Provider;
        use alloy_transport::Transport;
        use alloy_rpc_types::Log;
        use std::ops::Deref;
        use gadget_sdk::events_watcher::evm::Config;

        /// Event handler for the function
        #[doc = "[`"]
        #[doc = #fn_name_string]
        #[doc = "`]"]
        pub struct #struct_name<T: Config> {
            pub contract_address: alloy_primitives::Address,
            pub provider: std::sync::Arc<T::P>,
            #(#additional_params)*
        }

        #[automatically_derived]
        #[async_trait::async_trait]
        impl<T> gadget_sdk::events_watcher::evm::EventHandler<T> for #struct_name<T>
        where
            T: Config,
            #instance: Deref<Target = alloy_contract::ContractInstance<T::T, T::P, T::N>>,
        {
            type Contract = #instance;
            type Event = #event;

            async fn handle_event(
                &self,
                contract: &Self::Contract,
                (event, log): (Self::Event, Log),
            ) -> Result<(), gadget_sdk::events_watcher::Error> {
                Ok(())
            }
        }
    }
}
