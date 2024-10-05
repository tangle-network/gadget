use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::job::EventHandlerArgs;

pub(crate) fn generate_eigenlayer_event_handler(
    fn_name_string: &str,
    struct_name: &Ident,
    event_handler: &EventHandlerArgs,
    params_tokens: &[TokenStream],
    additional_params: &[TokenStream],
    fn_call: &TokenStream,
    event_listener_call: &TokenStream,
) -> TokenStream {
    let instance_base = event_handler.instance().unwrap();
    let instance_name = format_ident!("{}Instance", instance_base);
    let instance_wrapper_name = format_ident!("{}InstanceWrapper", instance_base);
    let ev = event_handler.event().unwrap();
    let event_converter = event_handler.event_converter().unwrap();
    let callback = event_handler.callback().unwrap();

    quote! {
        /// Event handler for the function
        #[doc = "[`"]
        #[doc = #fn_name_string]
        #[doc = "`]"]
        pub struct #struct_name {
            #(#additional_params)*
        }

        #[derive(Debug, Clone)]
        pub struct #instance_wrapper_name<T, P> {
            instance: #instance_base::#instance_name<T, P>,
            contract_instance: OnceLock<ContractInstance<T, P, alloy_network::Ethereum>>,
        }

        impl<T, P> #instance_wrapper_name<T, P>
        where
            T: alloy_transport::Transport + Clone + Send + Sync,
            P: alloy_provider::Provider<T, Ethereum> + Clone + Send + Sync,
        {
            /// Constructor for creating a new [`#instance_wrapper_name`].
            pub fn new(instance: #instance_base::#instance_name<T, P>) -> Self {
                #event_listener_call
                Self {
                    instance,
                    contract_instance: OnceLock::new(),
                }
            }

            /// Returns the provider of the [`ContractInstance`].
            pub fn provider(&self) -> &P {
                self.instance.provider()
            }

            /// Returns the address of the [`ContractInstance`].
            pub fn address(&self) -> &Address {
                self.instance.address()
            }

            /// Lazily creates the [`ContractInstance`] if it does not exist, otherwise returning a reference to it.
            // TODO: Remove Unwraps
            #[allow(clippy::clone_on_copy)]
            fn get_contract_instance(&self) -> &ContractInstance<T, P, Ethereum> {
                self.contract_instance.get_or_init(|| {
                    let instance_string = stringify!(#instance_base);
                    let abi_path = format!("./../blueprints/incredible-squaring/contracts/out/{}.sol/{}.json", instance_string, instance_string);
                    let json_str = std::fs::read_to_string(&abi_path).unwrap();
                    let json: Value = serde_json::from_str(&json_str).unwrap();
                    let json_abi = json["abi"].clone();
                    let abi_location = alloy_contract::Interface::new(JsonAbi::from_json_str(json_abi.to_string().as_str()).unwrap());
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
            T: alloy_transport::Transport + Clone + Send + Sync,
            P: alloy_provider::Provider<T, Ethereum> + Clone + Send + Sync,
        {
            type Target = ContractInstance<T, P, Ethereum>;

            /// Dereferences the [`#instance_wrapper_name`] to its [`ContractInstance`].
            fn deref(&self) -> &Self::Target {
                self.get_contract_instance()
            }
        }

        #[automatically_derived]
        #[async_trait::async_trait]
        impl<T> gadget_sdk::events_watcher::evm::EventHandler<T> for #struct_name
        where
            T: gadget_sdk::events_watcher::evm::Config,
        {
            type Contract = #instance_wrapper_name<T::TH, T::PH>;
            type Event = #ev;

            async fn handle_event(
                &self,
                contract: &Self::Contract,
                (event, log): (Self::Event, alloy_rpc_types::Log),
            ) -> Result<(), gadget_sdk::events_watcher::Error> {
                use alloy_provider::Provider;
                use alloy_sol_types::SolEvent;
                use alloy_sol_types::SolInterface;

                static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                if !ONCE.load(std::sync::atomic::Ordering::Relaxed) {
                    ONCE.store(true, std::sync::atomic::Ordering::Relaxed);
                    #event_listener_call
                }

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

                // let tx = contract.provider().send_raw_transaction(call.abi_encode().as_ref()).await.unwrap();
                // let receipt = tx.get_receipt().await.unwrap();
                // info!("SUBMITTED JOB RESULT: {:?}", receipt);

                info!("SUCCESSFULLY SUBMITTED JOB RESULT");

                Ok(())
            }
        }

        pub struct EigenlayerEventWatcher<T: Config> {
            contract_address: Address,
            provider: T::PH,
            handlers: Vec<Box<dyn gadget_sdk::events_watcher::evm::EventHandler<T, Contract = #instance_wrapper_name<T::TH, T::PH>, Event = #instance_base::NewTaskCreated> + Send + Sync>>,
            _phantom: std::marker::PhantomData<T>,
        }

        impl<T: Config> EigenlayerEventWatcher<T> {
            pub fn new(contract_address: Address, provider: T::PH) -> Self {
                Self {
                    contract_address,
                    provider,
                    handlers: vec![
                        Box::new(
                            #struct_name {}
                        )
                    ],
                    _phantom: std::marker::PhantomData,
                }
            }
        }

        impl<T: Config> EventWatcher<T> for EigenlayerEventWatcher<T> {
            const TAG: &'static str = "eigenlayer";
            type Contract = #instance_wrapper_name<T::TH, T::PH>;
            type Event = #instance_base::NewTaskCreated;
            const GENESIS_TX_HASH: FixedBytes<32> = FixedBytes([0; 32]);

            fn contract(&mut self) -> Self::Contract {
                let instance = #instance_base::#instance_name::new(
                    self.contract_address,
                    self.provider.clone()
                );
                #instance_wrapper_name::new(instance)
            }

            fn handlers(&self) -> &Vec<Box<dyn gadget_sdk::events_watcher::evm::EventHandler<T, Contract = #instance_wrapper_name<T::TH, T::PH>, Event = #instance_base::NewTaskCreated> + Send + Sync>> {
                &self.handlers
            }
        }
    }
}
