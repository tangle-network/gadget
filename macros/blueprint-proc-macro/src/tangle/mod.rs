pub mod event_listener;

use gadget_blueprint_proc_macro_core::FieldType;
use quote::{format_ident, quote};
use syn::Ident;

#[allow(clippy::too_many_lines)]
pub fn field_type_to_param_token(ident: &Ident, t: &FieldType) -> proc_macro2::TokenStream {
    match t {
        FieldType::Void => unreachable!("void type should not be in params"),
        FieldType::Bool => {
            quote! { let Some(Field::Bool(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Uint8 => {
            quote! { let Some(Field::Uint8(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Int8 => {
            quote! { let Some(Field::Int8(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Uint16 => {
            quote! { let Some(Field::Uint16(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Int16 => {
            quote! { let Some(Field::Int16(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Uint32 => {
            quote! { let Some(Field::Uint32(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Int32 => {
            quote! { let Some(Field::Int32(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Uint64 => {
            quote! { let Some(Field::Uint64(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Int64 => {
            quote! { let Some(Field::Int64(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Uint128 => {
            quote! { let Some(Field::Uint128(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::U256 => {
            quote! { let Some(Field::U256(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Int128 => {
            quote! { let Some(Field::Int128(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Float64 => {
            quote! { let Some(Field::Float64(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::String => {
            let inner_ident = format_ident!("{}_inner", ident);
            quote! {
                let Some(Field::String(BoundedString(BoundedVec(#inner_ident)))) = args_iter.next() else { continue; };
                // Convert the BoundedVec to a String
                let #ident = match String::from_utf8(#inner_ident) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!("failed to convert bytes to a valid utf8 string: {e}");
                        use gadget_sdk::events_watcher::Error;
                        return Err(Error::Handler(Box::new(e)));
                    }
                };
            }
        }
        FieldType::Bytes => {
            quote! { let Some(Field::Bytes(BoundedVec(#ident))) = args_iter.next() else { continue; }; }
        }
        FieldType::Optional(t_x) => {
            let inner_ident = format_ident!("{}_inner", ident);
            let x_ident = format_ident!("{}_option", ident);
            let x_inner = field_type_to_param_token(&x_ident, t_x);
            let inner = quote! {
                let Some(#inner_ident) = args_iter.next() else {  continue; };
            };
            quote! {
                #inner
                let #ident = match #inner_ident {
                    _ => {
                        #x_inner
                        Some(#x_ident)
                    },
                    Field::None => None,
                };
            }
        }
        FieldType::Array(_, _) => todo!("Handle array"),
        FieldType::List(_) => {
            let inner_ident = format_ident!("{}_inner", ident);
            let inner = quote! {
                let Some(Field::List(BoundedVec(#inner_ident))) = args_iter.next() else { continue; };
            };

            quote! {
                #inner
                let #ident = #inner_ident
                    .into_iter()
                    .map(|item| item.0)
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
                    let inner_token = field_type_to_param_token(&inner_ident, field_type);
                    quote! {
                        #inner_token
                        #field_ident: #inner_ident,
                    }
                })
                .collect();

            quote! {
                let Some(Field::Struct(#ident)) = args_iter.next() else { continue; };
                let mut #ident = #ident.into_iter();
                #(#field_tokens)*
                let #ident = #struct_ident {
                    #(#field_tokens)*
                };
            }
        }

        FieldType::AccountId => {
            quote! { let Some(Field::AccountId(#ident)) = args_iter.next() else { continue; }; }
        }
    }
}

pub fn field_type_to_result_token(ident: &Ident, t: &FieldType) -> proc_macro2::TokenStream {
    match t {
        FieldType::Void => quote! {},
        FieldType::Bool => quote! { result.push(Field::Bool(#ident)); },
        FieldType::Uint8 => quote! { result.push(Field::Uint8(#ident)); },
        FieldType::Int8 => quote! { result.push(Field::Int8(#ident)); },
        FieldType::Uint16 => quote! { result.push(Field::Uint16(#ident)); },
        FieldType::Int16 => quote! { result.push(Field::Int16(#ident)); },
        FieldType::Uint32 => quote! { result.push(Field::Uint32(#ident)); },
        FieldType::Int32 => quote! { result.push(Field::Int32(#ident)); },
        FieldType::Uint64 => quote! { result.push(Field::Uint64(#ident)); },
        FieldType::Int64 => quote! { result.push(Field::Int64(#ident)); },
        FieldType::Uint128 => quote! { result.push(Field::Uint128(#ident)); },
        FieldType::U256 => quote! { result.push(Field::U256(#ident)); },
        FieldType::Int128 => quote! { result.push(Field::Int128(#ident)); },
        FieldType::Float64 => quote! { result.push(Field::Float64(#ident)); },
        FieldType::String => {
            quote! { result.push(Field::String(BoundedString(BoundedVec(#ident.into_bytes())))); }
        }
        FieldType::Bytes => quote! { result.push(Field::Bytes(BoundedVec(#ident))); },
        FieldType::Optional(t_x) => {
            let v_ident = format_ident!("v");
            let tokens = field_type_to_result_token(&v_ident, t_x);
            quote! {
                match #ident {
                    Some(v) => #tokens,
                    None => result.push(Field::None),
                }
            }
        }
        FieldType::Array(_, _) => todo!("Handle array"),
        FieldType::List(t_x) => {
            let inner_ident = format_ident!("{}_inner", ident);
            let field = match **t_x {
                FieldType::Void => unreachable!(),
                FieldType::Bool => quote! { Field::Bool(item) },
                FieldType::Uint8 => quote! { Field::Uint8(item) },
                FieldType::Int8 => quote! { Field::Int8(item) },
                FieldType::Uint16 => quote! { Field::Uint16(item) },
                FieldType::Int16 => quote! { Field::Int16(item) },
                FieldType::Uint32 => quote! { Field::Uint32(item) },
                FieldType::Int32 => quote! { Field::Int32(item) },
                FieldType::Uint64 => quote! { Field::Uint64(item) },
                FieldType::Int64 => quote! { Field::Int64(item) },
                FieldType::Uint128 => quote! { Field::Uint128(item) },
                FieldType::Int128 => quote! { Field::Int128(item) },
                FieldType::U256 => quote! { Field::U256(item) },
                FieldType::Float64 => quote! { Field::Float64(item) },
                FieldType::String => {
                    quote! { Field::String(BoundedString(BoundedVec(item.into_bytes()))) }
                }
                FieldType::Bytes => quote! { Field::Bytes(BoundedVec(item)) },
                FieldType::Optional(_) => todo!("handle optionals into lists"),
                FieldType::Array(_, _) => todo!("handle arrays into lists"),
                FieldType::List(_) => todo!("handle nested lists"),
                FieldType::Struct(_, _) => todo!("handle nested structs"),
                FieldType::AccountId => quote! { Field::AccountId(item) },
            };
            let inner = quote! {
               let #inner_ident = #ident.into_iter().map(|item| #field).collect::<Vec<_>>();
            };

            quote! {
                #inner
                result.push(Field::List(BoundedVec(#inner_ident)));
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
                        let field_name = BoundedString::<C::MaxFieldsSize>::from(#field_name);
                        let field_value = Box::new(#inner_ident);
                        fields_vec.push((field_name, field_value));
                        #inner_token
                    }
                })
                .collect();

            quote! {
                #(#field_tokens)*
                let struct_name = BoundedString::<C::MaxFieldsSize>::from(#name);
                let fields_vec = vec![#(#field_tokens),*];
                result.push(Field::Struct(struct_name, BoundedVec(fields_vec)));
            }
        }
        FieldType::AccountId => {
            quote! { result.push(Field::AccountId(#ident)); }
        }
    }
}
