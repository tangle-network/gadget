use std::any::Any;

use gadget_blueprint_proc_macro_core::FieldType;
use quote::{format_ident, quote};
use syn::{Ident, Type};

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
        FieldType::Int128 => {
            quote! { let Some(Field::Int128(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::Float64 => {
            quote! { let Some(Field::Float64(#ident)) = args_iter.next() else { continue; }; }
        }
        FieldType::String => {
            quote! { let Some(Field::String(#ident)) = args_iter.next() else { continue; }; }
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
        FieldType::Int128 => quote! { result.push(Field::Int128(#ident)); },
        FieldType::Float64 => quote! { result.push(Field::Float64(#ident)); },
        FieldType::String => quote! { result.push(Field::String(#ident)); },
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
                FieldType::Float64 => quote! { Field::Float64(item) },
                FieldType::String => quote! { Field::String(item) },
                FieldType::Bytes => quote! { Field::Bytes(BoundedVec(item)) },
                FieldType::Optional(_) => todo!("handle optionals into lists"),
                FieldType::Array(_, _) => todo!("handle arrays into lists"),
                FieldType::List(_) => todo!("handle nested lists"),
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
        FieldType::AccountId => {
            quote! { result.push(Field::AccountId(#ident)); }
        }
    }
}

/// Convert a `snake_case` string to `PascalCase`
pub fn pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

pub fn ident_to_field_type(ident: &Ident) -> syn::Result<FieldType> {
    match ident.to_string().as_str() {
        "u8" => Ok(FieldType::Uint8),
        "u16" => Ok(FieldType::Uint16),
        "u32" => Ok(FieldType::Uint32),
        "u64" => Ok(FieldType::Uint64),
        "i8" => Ok(FieldType::Int8),
        "i16" => Ok(FieldType::Int16),
        "i32" => Ok(FieldType::Int32),
        "i64" => Ok(FieldType::Int64),
        "u128" => Ok(FieldType::Uint128),
        "i128" => Ok(FieldType::Int128),
        "f64" => Ok(FieldType::Float64),
        "bool" => Ok(FieldType::Bool),
        "String" => Ok(FieldType::String),
        "Bytes" => Ok(FieldType::Bytes),
        "AccountId" => Ok(FieldType::AccountId),
        _ => Err(syn::Error::new_spanned(ident, "unsupported type")),
    }
}

pub fn type_to_field_type(ty: &Type) -> syn::Result<FieldType> {
    match ty {
        Type::Array(_) => Err(syn::Error::new_spanned(ty, "TODO: support arrays")),
        Type::Path(inner) => path_to_field_type(&inner.path),
        _ => Err(syn::Error::new_spanned(ty, "unsupported type")),
    }
}

pub fn path_to_field_type(path: &syn::Path) -> syn::Result<FieldType> {
    // take the last segment of the path
    let seg = &path
        .segments
        .last()
        .ok_or_else(|| syn::Error::new_spanned(path, "path must have at least one segment"))?;
    let ident = &seg.ident;
    let args = &seg.arguments;
    match args {
        syn::PathArguments::None => ident_to_field_type(ident),
        // Support for Vec<T> where T is a simple type
        syn::PathArguments::AngleBracketed(inner) if ident.eq("Vec") && inner.args.len() == 1 => {
            let inner_arg = &inner.args[0];
            if let syn::GenericArgument::Type(inner_ty) = inner_arg {
                let inner_type = type_to_field_type(inner_ty)?;
                match inner_type {
                    FieldType::Uint8 => Ok(FieldType::Bytes),
                    others => Ok(FieldType::List(Box::new(others))),
                }
            } else {
                Err(syn::Error::new_spanned(
                    inner_arg,
                    "unsupported complex type",
                ))
            }
        }
        // Support for Option<T> where T is a simple type
        syn::PathArguments::AngleBracketed(inner)
            if ident.eq("Option") && inner.args.len() == 1 =>
        {
            let inner_arg = &inner.args[0];
            if let syn::GenericArgument::Type(inner_ty) = inner_arg {
                let inner_type = type_to_field_type(inner_ty)?;
                Ok(FieldType::Optional(Box::new(inner_type)))
            } else {
                Err(syn::Error::new_spanned(
                    inner_arg,
                    "unsupported complex type",
                ))
            }
        }
        // Support for Result<T, E> where T is a simple type
        syn::PathArguments::AngleBracketed(inner) if ident.eq("Result") => {
            let inner_arg = &inner.args[0];
            if let syn::GenericArgument::Type(inner_ty) = inner_arg {
                let inner_type = type_to_field_type(inner_ty)?;
                Ok(inner_type)
            } else {
                Err(syn::Error::new_spanned(
                    inner_arg,
                    "unsupported complex type",
                ))
            }
        }
        _ => Err(syn::Error::new_spanned(args, "unsupported complex type")),
    }
}
