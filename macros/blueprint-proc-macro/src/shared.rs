use crate::job::{IsResultType, ResultsKind};
use gadget_blueprint_proc_macro_core::FieldType;
use indexmap::IndexMap;
use quote::{quote, ToTokens};
use syn::{Ident, Signature, Type};

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
        "U256" => Ok(FieldType::U256),
        "i128" => Ok(FieldType::Int128),
        "f64" => Ok(FieldType::Float64),
        "bool" => Ok(FieldType::Bool),
        "String" => Ok(FieldType::String),
        "Bytes" => Ok(FieldType::Bytes),
        "AccountId" => Ok(FieldType::AccountId),
        _ => Err(syn::Error::new_spanned(ident, "unsupported field type")),
    }
}

pub fn type_to_field_type(ty: &Type) -> syn::Result<FieldType> {
    match ty {
        Type::Array(arr) => {
            let elem_type = type_to_field_type(&arr.elem)?;
            // convert arr.len expr to u64
            let len = match arr.len.to_token_stream().to_string().parse::<u64>() {
                Ok(l) => l,
                Err(_) => {
                    return Err(syn::Error::new_spanned(
                        &arr.len,
                        "array length must be a constant",
                    ))
                }
            };
            Ok(FieldType::Array(len, Box::new(elem_type)))
        }
        Type::Path(inner) => path_to_field_type(&inner.path),
        Type::Reference(type_reference) => type_to_field_type(&type_reference.elem),
        Type::Tuple(tuple) => {
            let mut ret = vec![];
            for elem in &tuple.elems {
                let elem_type = type_to_field_type(elem)?;
                ret.push(elem_type);
            }
            Ok(FieldType::Tuple(ret))
        }
        _ => Err(syn::Error::new_spanned(
            ty,
            "unsupported type (type_to_field_type)",
        )),
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
        syn::PathArguments::None => {
            match ident_to_field_type(ident) {
                Ok(field_type) => Ok(field_type),
                Err(_) => {
                    // Assume it's a custom struct if it's not a known type
                    Ok(FieldType::Struct(ident.to_string(), Vec::new()))
                }
            }
        }
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
        syn::PathArguments::AngleBracketed(inner)
            if ident.eq("Result") && !inner.args.is_empty() =>
        {
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
        syn::PathArguments::Parenthesized(_) => Err(syn::Error::new_spanned(
            args,
            "unsupported parenthesized arguments",
        )),
        // Support for SomeConcreteType<T,V, K, ...> where T, V, K is a simple type
        syn::PathArguments::AngleBracketed(inner) if !inner.args.is_empty() => {
            let mut ret = vec![];
            for inner_arg in &inner.args {
                if let syn::GenericArgument::Type(inner_ty) = inner_arg {
                    let inner_type = type_to_field_type(inner_ty)?;
                    ret.push((inner_ty.to_token_stream().to_string(), Box::new(inner_type)))
                } else {
                    return Err(syn::Error::new_spanned(inner_arg, "unsupported type param"));
                }
            }
            Ok(FieldType::Struct(ident.to_string(), ret))
        }

        syn::PathArguments::AngleBracketed(_) => {
            Err(syn::Error::new_spanned(args, "unsupported complex type"))
        }
    }
}

/// Returns the set of arguments which are not job-related arguments. These typically go into the
/// autogenerated job struct
pub fn get_non_job_arguments(
    param_map: &IndexMap<Ident, Type>,
    job_params: &[Ident],
) -> IndexMap<Ident, Type> {
    param_map
        .clone()
        .into_iter()
        .filter(|r| !job_params.contains(&r.0))
        .collect::<IndexMap<Ident, Type>>()
}

pub(crate) trait MacroExt {
    fn result_to_field_types(&self, result: &Type) -> syn::Result<Vec<FieldType>> {
        match self.return_type() {
            ResultsKind::Infered => type_to_field_type(result).map(|x| vec![x]),
            ResultsKind::Types(types) => {
                let xs = types
                    .iter()
                    .map(type_to_field_type)
                    .collect::<syn::Result<Vec<_>>>()?;
                Ok(xs)
            }
        }
    }

    fn return_type(&self) -> &ResultsKind;
}

pub(crate) fn param_types(sig: &Signature) -> syn::Result<IndexMap<Ident, Type>> {
    // Ensures that no duplicate parameters have been given
    let mut param_types = IndexMap::new();
    for input in &sig.inputs {
        if let syn::FnArg::Typed(arg) = input {
            if let syn::Pat::Ident(pat_ident) = &*arg.pat {
                let ident = &pat_ident.ident;
                let ty = &*arg.ty;
                let added = param_types.insert(ident.clone(), ty.clone());
                if added.is_some() {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "tried to add the same field twice",
                    ));
                }
            }
        }
    }

    Ok(param_types)
}

pub fn get_return_type_wrapper(return_type: &Type) -> proc_macro2::TokenStream {
    if return_type.is_result_type() {
        quote! { res.map_err(|err| gadget_sdk::Error::Other(err.to_string())) }
    } else {
        quote! { Ok(res) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_works() {
        let input = [
            "hello_world",
            "keygen",
            "_internal_function",
            "cggmp21_sign",
        ];
        let expected = ["HelloWorld", "Keygen", "InternalFunction", "Cggmp21Sign"];

        for (i, e) in input.iter().zip(expected.iter()) {
            assert_eq!(pascal_case(i), *e);
        }
    }
}
