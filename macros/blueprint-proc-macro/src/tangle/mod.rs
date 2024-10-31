use gadget_blueprint_proc_macro_core::FieldType;
use quote::{format_ident, quote};
use syn::Ident;

#[allow(clippy::too_many_lines)]
pub fn field_type_to_param_token(
    ident: &Ident,
    t: &FieldType,
    panic_on_decode_fail: bool,
) -> proc_macro2::TokenStream {
    let else_block = if panic_on_decode_fail {
        quote! {
            panic!("Failed to decode the field");
        }
    } else {
        quote! {
            return Ok(vec![]);
        }
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
            let x_inner = field_type_to_param_token(&x_ident, t_x, panic_on_decode_fail);
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
        FieldType::Array(_, _) => todo!("Handle array"),
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
        FieldType::Struct(name, fields) => {
            let struct_ident = format_ident!("{}", name);
            let field_tokens: Vec<_> = fields
                .iter()
                .map(|(field_name, field_type)| {
                    let field_ident = format_ident!("{}", field_name);
                    let inner_ident = format_ident!("{}_{}", ident, field_name);
                    let inner_token =
                        field_type_to_param_token(&inner_ident, field_type, panic_on_decode_fail);
                    quote! {
                        #inner_token
                        #field_ident: #inner_ident,
                    }
                })
                .collect();

            quote! {
                let Some(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Struct(#ident)) = args_iter.next() else { #else_block };
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
