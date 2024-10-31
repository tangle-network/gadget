use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::job::EventListenerArgs;

pub(crate) fn get_evm_instance_data(
    event_handler: &EventListenerArgs,
) -> (Ident, Ident, Ident, TokenStream) {
    let instance_base = event_handler.instance().unwrap();
    let instance_name = format_ident!("{}Instance", instance_base);
    let instance_wrapper_name = format_ident!("{}InstanceWrapper", instance_base);
    let instance = quote! { #instance_base::#instance_name<T::TH, T::PH, alloy_network::Ethereum> };

    (
        instance_base,
        instance_name,
        instance_wrapper_name,
        instance,
    )
}

pub(crate) fn generate_evm_event_handler(
    struct_name: &Ident,
    event_handler: &EventListenerArgs,
    params_tokens: &[TokenStream],
    fn_call: &TokenStream,
) -> TokenStream {
    let event = event_handler
        .get_event_listener()
        .event
        .as_ref()
        .expect("Event type must be specified");
    let event_converter = event_handler
        .get_event_listener()
        .pre_processor
        .as_ref()
        .unwrap();
    let _callback = event_handler
        .get_event_listener()
        .post_processor
        .as_ref()
        .unwrap();
    let abi_string = event_handler
        .get_event_listener()
        .evm_args
        .as_ref()
        .and_then(|r| r.abi.clone())
        .expect("ABI String must exist");

    quote! {
        impl<T: gadget_sdk::event_listener::evm_contracts::EthereumContractBound> gadget_sdk::event_listener::evm_contracts::EvmContractInstance<T> for #struct_name <T> {
            fn get_instance(&self) -> &alloy_contract::ContractInstance<T::TH, T::PH, alloy_network::Ethereum> {
                self.deref()
            }
        }

        impl<T: gadget_sdk::event_listener::evm_contracts::EthereumContractBound> Deref for #struct_name <T>
        {
            type Target = alloy_contract::ContractInstance<T::TH, T::PH, alloy_network::Ethereum>;
            fn deref(&self) -> &Self::Target {
                self.contract_instance.get_or_init(|| {
                    let abi_location = alloy_contract::Interface::new(alloy_json_abi::JsonAbi::from_json_str(&#abi_string).unwrap());
                    alloy_contract::ContractInstance::new(self.contract.address().clone(), self.contract.provider().clone(), abi_location )
                })
            }
        }

        #[automatically_derived]
        #[gadget_sdk::async_trait::async_trait]
        impl<T: gadget_sdk::event_listener::evm_contracts::EthereumContractBound> gadget_sdk::events_watcher::evm::EvmEventHandler<T> for #struct_name <T>
        {
            type Event = #event;
            async fn handle(&self, log: &gadget_sdk::alloy_rpc_types::Log, event: &Self::Event) -> Result<(), gadget_sdk::events_watcher::Error> {
                use alloy_provider::Provider;
                use alloy_sol_types::SolEvent;
                use alloy_sol_types::SolInterface;
                let contract = &self.contract;
                let decoded: alloy_primitives::Log<Self::Event> = <Self::Event as SolEvent>::decode_log(&log.inner, true)?;
                let (_, index) = decoded.topics();
                let inputs = #event_converter(decoded.data, index);

                // Apply the function
                #(#params_tokens)*
                #fn_call;
                Ok(())
            }
        }

        impl<T: gadget_sdk::event_listener::evm_contracts::EthereumContractBound> gadget_sdk::event_listener::markers::IsEvm for #struct_name <T> {}
    }
}
