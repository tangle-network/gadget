use indexmap::IndexMap;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Type};

use crate::job::{declared_params_to_field_types, EventListenerArgs};
use crate::shared::get_non_job_arguments;

pub(crate) fn get_evm_instance_data(
    event_handler: &EventListenerArgs,
) -> (Ident, Ident, Ident, TokenStream) {
    let instance_base = event_handler.instance().unwrap();
    let instance_name = format_ident!("{}Instance", instance_base);
    let instance_wrapper_name = format_ident!("{}InstanceWrapper", instance_base);
    let instance = quote! { #instance_base::#instance_name<gadget_sdk::event_listener::evm::contracts::BoxTransport, gadget_sdk::event_listener::evm::contracts::AlloyRootProvider, gadget_sdk::event_listener::evm::contracts::Ethereum> };

    (
        instance_base,
        instance_name,
        instance_wrapper_name,
        instance,
    )
}

pub(crate) fn generate_evm_specific_impl(
    struct_name: &Ident,
    event_listener_args: &EventListenerArgs,
    param_map: &IndexMap<Ident, Type>,
    job_params: &[Ident],
) -> TokenStream {
    let abi_string = event_listener_args
        .get_event_listener()
        .evm_args
        .as_ref()
        .and_then(|r| r.abi.clone())
        .expect("ABI String must exist");

    let non_job_param_map = get_non_job_arguments(param_map, job_params);
    let mut new_function_signature = vec![];
    let mut constructor_args = vec![];

    let (_, _, _, instance_name) = get_evm_instance_data(event_listener_args);

    // Push in the contract
    new_function_signature.push(quote! {
        contract: #instance_name,
    });
    constructor_args.push(quote! {
        contract,
        contract_instance: Default::default(),
    });

    for (field_name, ty) in non_job_param_map {
        new_function_signature.push(quote! {
            #field_name: #ty,
        });
        constructor_args.push(quote! {
            #field_name,
        })
    }

    quote! {
        impl #struct_name {
            pub fn new(#(#new_function_signature)*) -> Self {
                Self {
                    #(#constructor_args)*
                }
            }
        }

        impl Deref for #struct_name
        {
            type Target = gadget_sdk::event_listener::evm::contracts::AlloyContractInstance;
            fn deref(&self) -> &Self::Target {
                self.contract_instance.get_or_init(|| {
                    let abi_location = alloy_contract::Interface::new(alloy_json_abi::JsonAbi::from_json_str(&#abi_string).unwrap());
                    alloy_contract::ContractInstance::new(self.contract.address().clone(), self.contract.provider().clone(), abi_location )
                })
            }
        }

        impl gadget_sdk::event_listener::markers::IsEvm for #struct_name {}
    }
}

pub(crate) fn get_evm_job_processor_wrapper(
    params: &[Ident],
    param_types: &IndexMap<Ident, Type>,
    event_listeners: &EventListenerArgs,
    ordered_inputs: &mut Vec<TokenStream>,
    fn_name_ident: &Ident,
    asyncness: &TokenStream,
) -> TokenStream {
    let params =
        declared_params_to_field_types(params, param_types).expect("Failed to generate params");
    let params_tokens = event_listeners.get_param_name_tokenstream(&params, true);

    let job_processor_call = if params_tokens.is_empty() {
        let second_param = ordered_inputs.pop().expect("Expected a context");
        quote! {
            // If no args are specified, assume this job has no parameters and thus takes in the raw event
            #fn_name_ident (param0, #second_param) #asyncness .map_err(|err| gadget_sdk::Error::Other(err.to_string()))
        }
    } else {
        quote! {
            let inputs = param0;
            #(#params_tokens)*
            #fn_name_ident (#(#ordered_inputs)*) #asyncness .map_err(|err| gadget_sdk::Error::Other(err.to_string()))
        }
    };

    // We must type annotate param0 below as such: (_, _, _, ... ) using underscores for each input to
    // allow the rust type inferencer to count the number of inputs and correctly index them in the function call

    let inner_param_type = (0..params_tokens.len())
        .map(|_| quote!(_,))
        .collect::<Vec<_>>();

    quote! {
        move |param0: (#(#inner_param_type)*)| async move {
            #job_processor_call
        }
    }
}
