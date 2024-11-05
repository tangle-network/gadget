use gadget_blueprint_proc_macro_core::FieldType;
use indexmap::IndexMap;
use quote::ToTokens;
use syn::{Ident, Type};

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
        _ => Err(syn::Error::new_spanned(ident, "unsupported type")),
    }
}

pub fn type_to_field_type(ty: &Type) -> syn::Result<FieldType> {
    match ty {
        Type::Array(_) => Err(syn::Error::new_spanned(ty, "TODO: support arrays")),
        Type::Path(inner) => path_to_field_type(&inner.path),
        Type::Reference(type_reference) => type_to_field_type(&type_reference.elem),
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
