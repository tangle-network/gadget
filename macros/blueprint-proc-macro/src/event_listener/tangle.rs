use crate::job::{declared_params_to_field_types, EventListenerArgs};
use indexmap::IndexMap;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_additional_tangle_logic(struct_name: &Ident) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl gadget_sdk::event_listener::markers::IsTangle for #struct_name {}
    }
}

pub(crate) fn get_tangle_job_processor_wrapper(
    params: &[Ident],
    param_types: &IndexMap<Ident, Type>,
    event_listeners: &EventListenerArgs,
    ordered_inputs: &mut Vec<TokenStream>,
    fn_name_ident: &Ident,
    call_id_static_name: &Ident,
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
            let mut args_iter = param0.args.clone().into_iter();
            #(#params_tokens)*
            #fn_name_ident (#(#ordered_inputs)*) #asyncness .map_err(|err| gadget_sdk::Error::Other(err.to_string()))
        }
    };

    quote! {
        move |param0: gadget_sdk::event_listener::tangle::TangleEvent<_, _>| async move {
            if let Some(call_id) = param0.call_id {
                #call_id_static_name.store(call_id, std::sync::atomic::Ordering::Relaxed);
            }

            #job_processor_call
        }
    }
}
