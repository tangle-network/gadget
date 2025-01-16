use crate::job::args::EventListenerArgs;
use crate::job::declared_params_to_field_types;
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
) -> syn::Result<TokenStream> {
    let mut non_job_param_map = get_non_job_arguments(param_map, job_params);
    let mut new_function_signature = vec![];
    let mut constructor_args = vec![];

    if event_listener_args.get_event_listener().is_raw() {
        // TODO: task 001: find better way to identify which ident is the raw event
        // remove the 0th element
        let _ = non_job_param_map.shift_remove_index(0);
    }

    let env_type = quote! { gadget_macros::ext::config::GadgetConfiguration };

    // Push the expected types
    new_function_signature.push(quote! {
        env: &#env_type,
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

    Ok(quote! {
        // TODO: No Box<dyn Error>
        impl #struct_name {
            /// Create a new
            #[doc = "[`"]
            #[doc = #struct_name_as_literal]
            #[doc = "`]"]
            /// instance
            /// # Errors
            ///
            /// - The client fails to connect
            /// - The signer is not found
            /// - The service ID is not found.
            pub async fn new(#(#new_function_signature)*) -> Result<Self, Box<dyn core::error::Error>> {
                use gadget_macros::ext::keystore::backends::tangle::TangleBackend as _;
                use gadget_macros::ext::keystore::backends::Backend as _;

                let client =
                    <#env_type as gadget_macros::ext::contexts::tangle::TangleClientContext>::tangle_client(env)
                        .await
                        .map_err(|e| Into::<Box<dyn core::error::Error>>::into(e))?;

                // TODO: Key IDs
                let keystore = <#env_type as gadget_macros::ext::contexts::keystore::KeystoreContext>::keystore(env);
                let public = keystore.first_local::<
                    gadget_macros::ext::crypto::sp_core::SpSr25519
                >().map_err(|_| Into::<Box<dyn core::error::Error>>::into(gadget_macros::ext::config::Error::NoSr25519Keypair))?;
                let pair = keystore.get_secret::<
                    gadget_macros::ext::crypto::sp_core::SpSr25519
                >(&public)?;
                let signer = gadget_macros::ext::crypto::tangle_pair_signer::TanglePairSigner::new(pair.0);

                let service_id = env.protocol_settings
                    .tangle()
                    .map_err(|e| Into::<Box<dyn core::error::Error>>::into(e))?
                    .service_id
                    .ok_or_else(|| Into::<Box<dyn core::error::Error>>::into(gadget_macros::ext::config::Error::MissingServiceId))?;

                Ok(Self {
                    #(#constructor_args)*
                })
            }
        }

        #[automatically_derived]
        impl gadget_macros::ext::event_listeners::core::marker::IsTangle for #struct_name {}
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn get_tangle_job_processor_wrapper(
    job_params: &[Ident],
    param_map: &IndexMap<Ident, Type>,
    event_listeners: &EventListenerArgs,
    ordered_inputs: &mut Vec<TokenStream>,
    fn_name_ident: &Ident,
    asyncness: &TokenStream,
    return_type: &Type,
    ctx_pos_in_ordered_inputs: usize,
) -> syn::Result<TokenStream> {
    let params = declared_params_to_field_types(job_params, param_map)?;
    let params_tokens = event_listeners.get_param_name_tokenstream(&params);

    let parameter_count = params.len();
    let parameter_count_const = quote! {
        const PARAMETER_COUNT: usize = #parameter_count;
    };

    let injected_context_var_name = quote! { injected_context };

    let injected_context = ordered_inputs[ctx_pos_in_ordered_inputs].clone();
    let call_id_injector = quote! {
        let mut #injected_context_var_name = #injected_context;
        if let Some(call_id) = tangle_event.call_id {
            gadget_macros::ext::contexts::services::ServicesContext::set_call_id(&mut #injected_context_var_name, call_id);
        }
    };

    // Clone to allow passing to the post-processor closure
    ordered_inputs[ctx_pos_in_ordered_inputs] = quote! { injected_context.clone() };

    let job_processor_call = if params_tokens.is_empty() {
        let second_param = ordered_inputs
            .pop()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "Context type required"))?;
        quote! {
            #call_id_injector
            // If no args are specified, assume this job has no parameters and thus takes in the raw event
            let res = #fn_name_ident (tangle_event, #second_param) #asyncness;
        }
    } else {
        quote! {
            #parameter_count_const

            if tangle_event.args.len() != PARAMETER_COUNT {
                return Err(
                    gadget_macros::ext::event_listeners::core::Error::BadArgumentDecoding(format!("Parameter count mismatch, got `{}`, expected `{PARAMETER_COUNT}`", tangle_event.args.len()))
                );
            }

            let mut args = tangle_event.args.clone().into_iter();
            #(#params_tokens)*

            #call_id_injector

            let res = #fn_name_ident (#(#ordered_inputs),*) #asyncness;
        }
    };

    let job_processor_call_return =
        get_return_type_wrapper(return_type, Some(injected_context_var_name));

    Ok(quote! {
        move |tangle_event: gadget_macros::ext::event_listeners::tangle::events::TangleEvent<_, _>| async move {

            #job_processor_call
            #job_processor_call_return
        }
    })
}
