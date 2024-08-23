use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, LitInt};

use crate::job::EventHandlerArgs;

pub(crate) fn generate_eigenlayer_event_handler(
    fn_name_string: &str,
    struct_name: &Ident,
    _job_id: &LitInt,
    event_handler: &EventHandlerArgs,
    params_tokens: &[TokenStream],
    _result_tokens: &[TokenStream],
    additional_params: &[TokenStream],
    fn_call: &TokenStream,
) -> TokenStream {
    let instance_base = event_handler.instance().unwrap();
    let instance_name = format_ident!("{}Instance", instance_base);
    let instance = quote! { #instance_base::#instance_name<T::T, T::P, T::N> };
    let event = event_handler.event().unwrap();
    let event_converter = event_handler.event_converter().unwrap();
    let callback = event_handler.callback().unwrap();

    quote! {
        use alloy_network::Network;
        use alloy_provider::Provider;
        use alloy_transport::Transport;
        use alloy_sol_types::SolEvent;
        use alloy_sol_types::SolInterface;
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
                (event, log): (Self::Event, alloy_rpc_types::Log),
            ) -> Result<(), gadget_sdk::events_watcher::Error> {
                // Convert the event to inputs
                let decoded: alloy_primitives::Log<Self::Event> = <Self::Event as SolEvent>::decode_log(&log.inner, true)?;
                // Convert the event to inputs using the event converter
                let inputs = #event_converter(decoded.data);

                // Apply the function
                #(#params_tokens)*
                #fn_call;

                // Call the callback with the job result
                let call = #callback(job_result);

                // Submit the transaction
                let tx = contract.provider().send_raw_transaction(call.abi_encode().as_ref()).await?;
                tx.watch().await?;
                Ok(())
            }
        }
    }
}
