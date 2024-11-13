use crate::job::{declared_params_to_field_types, EventListenerArgs};
use crate::shared::{get_non_job_arguments, get_return_type_wrapper};
use indexmap::IndexMap;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, Type};

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_tangle_specific_impl(
    struct_name: &Ident,
    param_map: &IndexMap<Ident, Type>,
    job_params: &[Ident],
    event_listener_args: &EventListenerArgs,
) -> TokenStream {
    let mut non_job_param_map = get_non_job_arguments(param_map, job_params);
    let mut new_function_signature = vec![];
    let mut constructor_args = vec![];

    if event_listener_args.get_event_listener().is_raw() {
        // TODO: task 001: find better way to identify which ident is the raw event
        // remove the 0th element
        let _ = non_job_param_map.shift_remove_index(0);
    }

    // Push the expected types
    new_function_signature.push(quote! {
        env: &gadget_sdk::config::gadget_config::GadgetConfiguration<gadget_sdk::parking_lot::RawRwLock>,
    });

    constructor_args.push(quote! {
        client,
        signer,
        service_id,
    });

    for (param_name, param_type) in non_job_param_map {
        new_function_signature.push(quote! {
            #param_name: #param_type,
        });

        constructor_args.push(quote! {
            #param_name,
        });
    }

    let struct_name_as_literal = struct_name.to_string();

    quote! {
        impl #struct_name {
            /// Create a new
            #[doc = "[`"]
            #[doc = #struct_name_as_literal]
            #[doc = "`]"]
            /// instance
            /// # Errors
            ///
            /// - `gadget_sdk::Error`: if the client fails to connect, the signer is not found, or
            /// the service ID is not found.
            pub async fn new(#(#new_function_signature)*) -> Result<Self, gadget_sdk::Error> {
                let client = env.client().await?;
                let signer = env.first_sr25519_signer()?;
                let service_id = env.service_id().ok_or_else(|| gadget_sdk::Error::Other("No service ID found in ENV".to_string()))?;

                Ok(Self {
                    #(#constructor_args)*
                })
            }
        }

        #[automatically_derived]
        impl gadget_sdk::event_listener::markers::IsTangle for #struct_name {}
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn get_tangle_job_processor_wrapper(
    job_params: &[Ident],
    param_map: &IndexMap<Ident, Type>,
    event_listeners: &EventListenerArgs,
    ordered_inputs: &mut Vec<TokenStream>,
    fn_name_ident: &Ident,
    call_id_static_name: &Ident,
    asyncness: &TokenStream,
    return_type: &Type,
) -> syn::Result<TokenStream> {
    let params = declared_params_to_field_types(job_params, param_map)?;
    let params_tokens = event_listeners.get_param_name_tokenstream(&params);

    let parameter_count = params.len();
    let parameter_count_const = quote! {
        const PARAMETER_COUNT: usize = #parameter_count;
    };

    let job_processor_call = if params_tokens.is_empty() {
        let second_param = ordered_inputs
            .pop()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "Context type required"))?;
        quote! {
            // If no args are specified, assume this job has no parameters and thus takes in the raw event
            let res = #fn_name_ident (param0, #second_param) #asyncness;
        }
    } else {
        quote! {
            #parameter_count_const

            if param0.args.len() != PARAMETER_COUNT {
                return Err(
                    ::gadget_sdk::Error::BadArgumentDecoding(format!("Parameter count mismatch, got `{}`, expected `{PARAMETER_COUNT}`", param0.args.len()))
                );
            }

            let mut args = param0.args.into_iter();
            #(#params_tokens)*
            let res = #fn_name_ident (#(#ordered_inputs)*) #asyncness;
        }
    };

    let job_processor_call_return = get_return_type_wrapper(return_type);

    Ok(quote! {
        move |param0: gadget_sdk::event_listener::tangle::TangleEvent<_, _>| async move {
            if let Some(call_id) = param0.call_id {
                #call_id_static_name.store(call_id, std::sync::atomic::Ordering::Relaxed);
            }

            #job_processor_call
            #job_processor_call_return
        }
    })
}
