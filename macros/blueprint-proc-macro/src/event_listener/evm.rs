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
    let instance = quote! { #instance_base::#instance_name<alloy_transport::BoxTransport, alloy_provider::RootProvider<alloy_transport::BoxTransport>, alloy_network::Ethereum> };

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
    let (instance_base, instance_name, instance_wrapper_name, _instance) =
        get_evm_instance_data(event_handler);
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
    let callback = event_handler
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
        #[derive(Debug, Clone)]
        pub struct #instance_wrapper_name<T, P> {
            instance: #instance_base::#instance_name<T, P>,
            contract_instance: OnceLock<alloy_contract::ContractInstance<T, P, alloy_network::Ethereum>>,
        }

        impl<T, P> From<#instance_base::#instance_name<T, P>> for #instance_wrapper_name<T, P>
        where
            T: alloy_transport::Transport + Clone + Send + Sync + 'static,
            P: alloy_provider::Provider<T> + Clone + Send + Sync + 'static {
            fn from(instance: #instance_base::#instance_name<T, P>) -> Self {
                Self::new(instance)
            }
        }

        impl<T, P> #instance_wrapper_name<T, P>
        where
            T: alloy_transport::Transport + Clone + Send + Sync + 'static,
            P: alloy_provider::Provider<T> + Clone + Send + Sync + 'static,
        {
            /// Constructor for creating a new [`#instance_wrapper_name`].
            pub fn new(instance: #instance_base::#instance_name<T, P>) -> Self {
                Self {
                    instance,
                    contract_instance: OnceLock::new(),
                }
            }

            /// Lazily creates the [`alloy_contract::ContractInstance`] if it does not exist, otherwise returning a reference to it.
            #[allow(clippy::clone_on_copy)]
            fn get_contract_instance(&self) -> &alloy_contract::ContractInstance<T, P, alloy_network::Ethereum> {
                self.contract_instance.get_or_init(|| {
                    let abi_location = alloy_contract::Interface::new(alloy_json_abi::JsonAbi::from_json_str(&#abi_string).unwrap());
                    alloy_contract::ContractInstance::new(
                        self.instance.address().clone(),
                        self.instance.provider().clone(),
                        abi_location,
                    )
                })
            }
        }


        impl<T, P> Deref for #instance_wrapper_name<T, P>
        where
            T: alloy_transport::Transport + Clone + Send + Sync + 'static,
            P: alloy_provider::Provider<T> + Clone + Send + Sync + 'static,
        {
           type Target = alloy_contract::ContractInstance<T, P, alloy_network::Ethereum>;

           /// Dereferences the [`#instance_wrapper_name`] to its [`alloy_contract::ContractInstance`].
           fn deref(&self) -> &Self::Target {
               self.get_contract_instance()
            }
        }


        #[automatically_derived]
        #[async_trait::async_trait]
        impl gadget_sdk::events_watcher::evm::EvmEventHandler for #struct_name
        where
            #instance_wrapper_name <alloy_transport::BoxTransport, alloy_provider::RootProvider<alloy_transport::BoxTransport>>: std::ops::Deref<Target = alloy_contract::ContractInstance<alloy_transport::BoxTransport, alloy_provider::RootProvider<alloy_transport::BoxTransport>, alloy_network::Ethereum>>,
        {
            type Contract = #instance_wrapper_name <alloy_transport::BoxTransport, alloy_provider::RootProvider<alloy_transport::BoxTransport>>;
            type Event = #event;
            const GENESIS_TX_HASH: alloy_primitives::FixedBytes<32> = alloy_primitives::FixedBytes([0; 32]);

            async fn handle(&self, log: &gadget_sdk::alloy_rpc_types::Log, event: &Self::Event) -> Result<(), gadget_sdk::events_watcher::Error> {
                use alloy_provider::Provider;
                use alloy_sol_types::SolEvent;
                use alloy_sol_types::SolInterface;

                let contract = &self.contract;

                // Convert the event to inputs
                let decoded: alloy_primitives::Log<Self::Event> = <Self::Event as SolEvent>::decode_log(&log.inner, true)?;

                let (_, index) = decoded.topics();
                // Convert the event to inputs using the event converter.
                // TODO: If no converter is provided, the #[job] must consume the
                // event directly, as specified in the `event = <EVENT>`.

                // let inputs = if let Some(converter) = #event_converter {
                //     converter(decoded.data)
                // } else {
                //     decoded.data
                // };
                let inputs = #event_converter(decoded.data, index);

                // Apply the function
                #(#params_tokens)*
                #fn_call;

                // Call the callback with the job result
                // TODO: Check if the callback is None
                // if let Some(cb) = #callback {
                //     let call = cb(job_result);
                //     // Submit the transaction
                //     let tx = contract.provider().send_raw_transaction(call.abi_encode().as_ref()).await?;
                //     tx.watch().await?;
                // }
                let call = #callback(job_result);
                // Submit the transaction
                // let tx = contract.provider().send_raw_transaction(call.abi_encode().as_ref()).await?;
                // tx.watch().await?;

                Ok(())
            }
        }

        impl gadget_sdk::event_listener::markers::IsEvm for #struct_name {}
    }
}
