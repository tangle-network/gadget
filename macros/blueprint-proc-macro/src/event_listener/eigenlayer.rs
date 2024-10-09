use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::job::EventHandlerArgs;

pub(crate) fn get_instance_data(
    event_handler: &EventHandlerArgs,
) -> (Ident, Ident, Ident, TokenStream) {
    let instance_base = event_handler.instance().unwrap();
    let instance_name = format_ident!("{}Instance", instance_base);
    let instance_wrapper_name = format_ident!("{}InstanceWrapper", instance_base);
    let instance = quote! { #instance_base::#instance_name<T::T, T::P, alloy_network::Ethereum> };

    (
        instance_base,
        instance_name,
        instance_wrapper_name,
        instance,
    )
}

pub(crate) fn generate_evm_event_handler(
    fn_name_string: &str,
    struct_name: &Ident,
    event_handler: &EventHandlerArgs,
    params_tokens: &[TokenStream],
    additional_params: &[TokenStream],
    fn_call: &TokenStream,
    event_listener_calls: &[TokenStream],
) -> TokenStream {
    let (instance_base, instance_name, instance_wrapper_name, _instance) =
        get_instance_data(event_handler);
    let ev = event_handler.event().unwrap();
    let event_converter = event_handler.event_converter().unwrap();
    let callback = event_handler.callback().unwrap();

    quote! {
        /// Event handler for the function
        #[doc = "[`"]
        #[doc = #fn_name_string]
        #[doc = "`]"]
        #[derive(Clone)]
        pub struct #struct_name <T: Clone + Send + Sync + gadget_sdk::events_watcher::evm::Config<N = Ethereum> +'static>{
            contract: #instance_wrapper_name<T::T, T::P>,
            #(#additional_params)*
        }

        #[derive(Debug, Clone)]
        pub struct #instance_wrapper_name<T, P> {
            instance: #instance_base::#instance_name<T, P>,
            contract_instance: OnceLock<ContractInstance<T, P, alloy_network::Ethereum>>,
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
            /// Lazily creates the [`ContractInstance`] if it does not exist, otherwise returning a reference to it.
            #[allow(clippy::clone_on_copy)]
            fn get_contract_instance(&self) -> &ContractInstance<T, P, Ethereum> {
                self.contract_instance.get_or_init(|| {
                    let instance_string = stringify!(instance_name);
                    let abi_path = format!("../contracts/out/{instance_string}.sol/{instance_string}.json");
                    let abi_location = alloy_contract::Interface::new(JsonAbi::from_json_str(&abi_path).unwrap());
                    ContractInstance::new(
                        self.instance.address().clone(),
                        self.instance.provider().clone(),
                        abi_location,
                    )
                })
            }
        }


        impl<T, P> Deref for #instance_wrapper_name<T, P>
        where
            T: Transport + Clone + Send + Sync + 'static,
            P: Provider<T> + Clone + Send + Sync + 'static,
        {
           type Target = ContractInstance<T, P, Ethereum>;

           /// Dereferences the [`#instance_wrapper_name`] to its [`ContractInstance`].
           fn deref(&self) -> &Self::Target {
               self.get_contract_instance()
            }
        }


        #[automatically_derived]
        #[async_trait::async_trait]
        impl<T> gadget_sdk::events_watcher::evm::EvmEventHandler<T> for #struct_name <T>
        where
            T: Clone + Send + Sync + gadget_sdk::events_watcher::evm::Config<N = Ethereum> +'static,
            #instance_wrapper_name <T::T, T::P>: std::ops::Deref<Target = alloy_contract::ContractInstance<T::T, T::P, T::N>>,
        {
            type Contract = #instance_wrapper_name <T::T, T::P>;
            type Event = #ev;
            const TAG: &'static str = "eigenlayer";
            const GENESIS_TX_HASH: FixedBytes<32> = FixedBytes([0; 32]);

            async fn init(&self) {
                #(#event_listener_calls)*
            }

            async fn handle(&self, log: &gadget_sdk::alloy_rpc_types::Log, event: &Self::Event) -> Result<(), gadget_sdk::events_watcher::Error> {
                use alloy_provider::Provider;
                use alloy_sol_types::SolEvent;
                use alloy_sol_types::SolInterface;

                let contract = &self.contract;

                // Convert the event to inputs
                let decoded: alloy_primitives::Log<Self::Event> = <Self::Event as SolEvent>::decode_log(&log.inner, true)?;
                // Convert the event to inputs using the event converter.
                // TODO: If no converter is provided, the #[job] must consume the
                // event directly, as specified in the `event = <EVENT>`.

                // let inputs = if let Some(converter) = #event_converter {
                //     converter(decoded.data)
                // } else {
                //     decoded.data
                // };
                let inputs = #event_converter(decoded.data);

                // Apply the function
                #(#params_tokens)*
                #fn_call;

                // Call the callback with the job result
                // TODO: Check if the callback is None
                // if let Some(cb) = #callback {
                //     let call = cb(job_result);
                //
                //     // Submit the transaction
                //     let tx = contract.provider().send_raw_transaction(call.abi_encode().as_ref()).await?;
                //     tx.watch().await?;
                // }
                let call = #callback(job_result);
                // Submit the transaction
                let tx = contract.provider().send_raw_transaction(call.abi_encode().as_ref()).await?;
                tx.watch().await?;

                Ok(())
            }
        }

        pub struct EigenlayerGadgetRunner<R: lock_api::RawRwLock> {
            pub env: GadgetConfiguration<R>,
            /// The EigenLayer Operator that registers to the AVS and completes given tasks
            pub operator: Option<Operator<NodeConfig, OperatorInfoService>>,
        }

        impl<R: lock_api::RawRwLock> EigenlayerGadgetRunner<R> {
            pub async fn new(env: GadgetConfiguration<R>) -> Self {
                Self {
                    env,
                    operator: None,
                }
            }

            pub fn set_operator(&mut self, operator: Operator<NodeConfig, OperatorInfoService>) {
                self.operator = operator.into();
            }
        }
    }
}
