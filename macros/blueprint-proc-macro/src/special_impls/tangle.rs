use crate::job::{declared_params_to_field_types, EventListenerArgs};
use crate::shared::{get_non_job_arguments, get_return_type_wrapper};
use gadget_blueprint_proc_macro_core::FieldType;
use indexmap::IndexMap;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
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
            let mut args_iter = param0.args.clone().into_iter();
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

#[allow(clippy::too_many_lines)]
pub fn field_type_to_param_token(ident: &Ident, t: &FieldType) -> TokenStream {
    let ident_str = ident.to_string();
    let field_type_str = format!("{:?}", t);

    let else_block = quote! {
        return Err(gadget_sdk::Error::BadArgumentDecoding(format!("Failed to decode the field {:?} to {:?}", #ident_str, #field_type_str)));
    };

    match t {
        FieldType::Void => unreachable!("void type should not be in params"),
        FieldType::Bool => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Bool(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::Uint8 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint8(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::Int8 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int8(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::Uint16 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint16(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::Int16 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int16(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::Uint32 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint32(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::Int32 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int32(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::Uint64 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint64(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::Int64 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int64(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::Uint128 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint128(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::U256 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::U256(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::Int128 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int128(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::Float64 => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Float64(#ident)) = args_iter.next() else { #else_block }; }
        }
        FieldType::String => {
            let inner_ident = format_ident!("{}_inner", ident);
            quote! {
                let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::String(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::BoundedString(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec(#inner_ident)))) = args_iter.next() else { #else_block };
                // Convert the BoundedVec to a String
                let #ident = match String::from_utf8(#inner_ident) {
                    Ok(s) => s,
                    Err(e) => {
                        ::gadget_sdk::warn!("failed to convert bytes to a valid utf8 string: {e}");
                        return Err(gadget_sdk::Error::Other(e.to_string()));
                    }
                };
            }
        }
        FieldType::Bytes => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Bytes(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec(#ident))) = args_iter.next() else { #else_block }; }
        }
        FieldType::Optional(t_x) => {
            let inner_ident = format_ident!("{}_inner", ident);
            let x_ident = format_ident!("{}_option", ident);
            let x_inner = field_type_to_param_token(&x_ident, t_x);
            let inner = quote! {
                let Some(#inner_ident) = args_iter.next() else {  #else_block; };
            };
            quote! {
                #inner
                let #ident = match #inner_ident {
                    _ => {
                        #x_inner
                        Some(#x_ident)
                    },
                    gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::None => None,
                };
            }
        }
        FieldType::Array(_len, ty) => {
            let inner_ident = format_ident!("{}_inner", ident);
            let inner = quote! {
                let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Array(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec(#inner_ident))) = args_iter.next() else { #else_block; };
            };

            let ty_variant_ident = format_ident!("{ty:?}");

            quote! {
                #inner
                let #ident = #inner_ident
                    .into_iter()
                    .map(|item| if let gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::<>:: #ty_variant_ident(val) = item {
                        val.0
                    } else {
                        panic!("Failed to decode the array");
                    })
                    .collect::<Vec<_>>();
            }
        }
        FieldType::List(ty) => {
            let inner_ident = format_ident!("{}_inner", ident);
            let inner = quote! {
                let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::List(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec(#inner_ident))) = args_iter.next() else { #else_block; };
            };

            let ty_variant_ident = format_ident!("{ty:?}");

            quote! {
                #inner
                let #ident = #inner_ident
                    .into_iter()
                    .map(|item| if let gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::<>:: #ty_variant_ident(val) = item {
                        val.0
                    } else {
                        panic!("Failed to decode the list");
                    })
                    .collect::<Vec<_>>();
            }
        }
        FieldType::Tuple(elements) => {
            let inner_tokens: Vec<_> = elements
                .iter()
                .enumerate()
                .map(|(i, ty)| {
                    let inner_ident = format_ident!("{}_{}", ident, i);
                    let inner_token = field_type_to_param_token(&inner_ident, ty);
                    quote! {
                        #inner_token
                        #inner_ident,
                    }
                })
                .collect();

            quote! {
                let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Tuple(#ident, ..)) = args_iter.next() else { #else_block };
                let mut #ident = #ident.into_iter();
                #(#inner_tokens)*
                let #ident = (#(#inner_tokens)*);
            }
        }
        FieldType::Struct(name, fields) => {
            let struct_ident = format_ident!("{}", name);
            let field_tokens: Vec<_> = fields
                .iter()
                .map(|(field_name, field_type)| {
                    let field_ident = format_ident!("{}", field_name);
                    let inner_ident = format_ident!("{}_{}", ident, field_name);
                    let inner_token = field_type_to_param_token(&inner_ident, field_type);
                    quote! {
                        #inner_token
                        #field_ident: #inner_ident,
                    }
                })
                .collect();

            quote! {
                let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Struct(#ident, ..)) = args_iter.next() else { #else_block };
                let mut #ident = #ident.into_iter();
                #(#field_tokens)*
                let #ident = #struct_ident {
                    #(#field_tokens)*
                };
            }
        }

        FieldType::AccountId => {
            quote! { let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::AccountId(#ident)) = args_iter.next() else { #else_block }; }
        }
    }
}
