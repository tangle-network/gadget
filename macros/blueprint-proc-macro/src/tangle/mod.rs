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

pub fn field_type_to_result_token(ident: &Ident, t: &FieldType) -> proc_macro2::TokenStream {
    match t {
        FieldType::Void => quote! {},
        FieldType::Bool => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Bool(#ident)); }
        }
        FieldType::Uint8 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint8(#ident)); }
        }
        FieldType::Int8 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int8(#ident)); }
        }
        FieldType::Uint16 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint16(#ident)); }
        }
        FieldType::Int16 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int16(#ident)); }
        }
        FieldType::Uint32 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint32(#ident)); }
        }
        FieldType::Int32 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int32(#ident)); }
        }
        FieldType::Uint64 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint64(#ident)); }
        }
        FieldType::Int64 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int64(#ident)); }
        }
        FieldType::Uint128 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint128(#ident)); }
        }
        FieldType::U256 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::U256(#ident)); }
        }
        FieldType::Int128 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int128(#ident)); }
        }
        FieldType::Float64 => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Float64(#ident)); }
        }
        FieldType::String => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::String(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::BoundedString(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec(#ident.into_bytes())))); }
        }
        FieldType::Bytes => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Bytes(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec(#ident))); }
        }
        FieldType::Optional(t_x) => {
            let v_ident = format_ident!("v");
            let tokens = field_type_to_result_token(&v_ident, t_x);
            quote! {
                match #ident {
                    Some(v) => #tokens,
                    None => result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::None),
                }
            }
        }
        FieldType::Array(_, _) => todo!("Handle array"),
        FieldType::List(t_x) => {
            let inner_ident = format_ident!("{}_inner", ident);
            let field = match **t_x {
                FieldType::Void => unreachable!(),
                FieldType::Bool => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Bool(item) }
                }
                FieldType::Uint8 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint8(item) }
                }
                FieldType::Int8 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int8(item) }
                }
                FieldType::Uint16 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint16(item) }
                }
                FieldType::Int16 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int16(item) }
                }
                FieldType::Uint32 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint32(item) }
                }
                FieldType::Int32 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int32(item) }
                }
                FieldType::Uint64 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint64(item) }
                }
                FieldType::Int64 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int64(item) }
                }
                FieldType::Uint128 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Uint128(item) }
                }
                FieldType::Int128 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Int128(item) }
                }
                FieldType::U256 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::U256(item) }
                }
                FieldType::Float64 => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Float64(item) }
                }
                FieldType::String => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::String(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::BoundedString(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec(item.into_bytes()))) }
                }
                FieldType::Bytes => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Bytes(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec(item)) }
                }
                FieldType::Optional(_) => todo!("handle optionals into lists"),
                FieldType::Array(_, _) => todo!("handle arrays into lists"),
                FieldType::List(_) => todo!("handle nested lists"),
                FieldType::Struct(_, _) => todo!("handle nested structs"),
                FieldType::AccountId => {
                    quote! { gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::AccountId(item) }
                }
            };
            let inner = quote! {
               let #inner_ident = #ident.into_iter().map(|item| #field).collect::<Vec<_>>();
            };

            quote! {
                #inner
                result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::List(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec(#inner_ident)));
            }
        }
        FieldType::Struct(name, fields) => {
            let field_tokens: Vec<_> = fields
                .iter()
                .map(|(field_name, field_type)| {
                    let field_ident = format_ident!("{}", field_name);
                    let inner_ident = format_ident!("{}_{}", ident, field_name);
                    let inner_token = field_type_to_result_token(&inner_ident, field_type);
                    quote! {
                        let #inner_ident = #ident.#field_ident;
                        let field_name = gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::BoundedString::<C::MaxFieldsSize>::from(#field_name);
                        let field_value = Box::new(#inner_ident);
                        fields_vec.push((field_name, field_value));
                        #inner_token
                    }
                })
                .collect();

            quote! {
                #(#field_tokens)*
                let struct_name = gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::BoundedString::<C::MaxFieldsSize>::from(#name);
                let fields_vec = vec![#(#field_tokens),*];
                result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::Struct(struct_name, gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec(fields_vec)));
            }
        }
        FieldType::AccountId => {
            quote! { result.push(gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field::AccountId(#ident)); }
        }
    }
}
