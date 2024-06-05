use std::collections::{BTreeMap, HashSet};

use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, Token, Type};

// Defines custom keywords
mod kw {
    syn::custom_keyword!(params);
    syn::custom_keyword!(result);
}

pub(crate) fn job_impl(args: &JobArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    // Extract function name and arguments
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let struct_name = format_ident!("{}Job", pascal_case(&fn_name_string));

    let syn::ReturnType::Type(_, result) = &input.sig.output else {
        return Err(syn::Error::new_spanned(
            &input.sig.output,
            "Function must have a return type",
        ));
    };

    let mut param_types = BTreeMap::new();
    for input in &input.sig.inputs {
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

    // Extract params and result types from args
    let params_type = args.params_to_token_stream(&param_types)?;
    let result_type = args.result_to_token_stream(result)?;
    // Generate the struct
    let gen = quote! {
        struct #struct_name {
            params: #params_type,
            result: #result_type,
        }

        #input
    };

    eprintln!("Generated code: {gen}");

    Ok(gen.into())
}

/// Convert a `snake_case` string to `PascalCase`
fn pascal_case(s: &str) -> String {
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

/// `JobArgs` type to handle parsing of attributes
pub(crate) struct JobArgs {
    params: Vec<Ident>,
    result: ResultsKind,
}

impl Parse for JobArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut params = Vec::new();
        let mut result = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::params) {
                let Params(p) = input.parse()?;
                params = p;
            } else if lookahead.peek(kw::result) {
                let Results(r) = input.parse()?;
                result = Some(r);
            } else if lookahead.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            } else {
                return Err(lookahead.error());
            }
        }

        if params.is_empty() {
            return Err(input.error("Missing 'params' argument in attribute"));
        }

        let result = result.ok_or_else(|| input.error("Missing 'result' argument in attribute"))?;

        if let ResultsKind::Types(ref r) = result {
            if r.is_empty() {
                return Err(input.error("Expected at least one parameter for the `result` attribute, or `_` to infer the type"));
            }
        }

        Ok(JobArgs { params, result })
    }
}

#[derive(Debug)]
struct Params(Vec<Ident>);

impl Parse for Params {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let _ = input.parse::<kw::params>();
        let content;
        let _ = syn::parenthesized!(content in input);
        let names = content.parse_terminated(Ident::parse_any, Token![,])?;
        let mut items = HashSet::new();
        let mut args = Vec::new();
        for name in names {
            if items.contains(&name) {
                return Err(syn::Error::new(
                    name.span(),
                    "tried to add the same field twice",
                ));
            }

            let inserted = items.insert(name.clone());
            assert!(inserted, "tried to add the same field twice");
            args.push(name);
        }
        Ok(Self(args))
    }
}

enum ResultsKind {
    Infered,
    Types(Vec<Type>),
}

impl std::fmt::Debug for ResultsKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Infered => write!(f, "Infered"),
            Self::Types(_) => write!(f, "Types"),
        }
    }
}

#[derive(Debug)]
struct Results(ResultsKind);

impl Parse for Results {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let _ = input.parse::<kw::result>();
        let content;
        let _ = syn::parenthesized!(content in input);
        let names = content.parse_terminated(Type::parse, Token![,])?;
        if names.is_empty() {
            return Err(syn::Error::new_spanned(
                names,
                "Expected at least one parameter",
            ));
        }
        if names.iter().any(|ty| matches!(ty, Type::Infer(_))) {
            // Infer the types from the retun type
            return Ok(Self(ResultsKind::Infered));
        }
        let mut items = Vec::new();
        for name in names {
            items.push(name);
        }
        Ok(Self(ResultsKind::Types(items)))
    }
}

impl JobArgs {
    fn params_to_token_stream(
        &self,
        param_types: &BTreeMap<Ident, Type>,
    ) -> syn::Result<proc_macro2::TokenStream> {
        let params = self
            .params
            .iter()
            .map(|ident| {
                param_types.get(ident).ok_or_else(|| {
                    syn::Error::new_spanned(ident, "parameter not declared in the function")
                })
            })
            .map(|ty| type_to_field_type(ty?))
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(quote! { (#(#params),*) })
    }

    fn result_to_token_stream(&self, result: &Type) -> syn::Result<proc_macro2::TokenStream> {
        match &self.result {
            ResultsKind::Infered => type_to_field_type(result),
            ResultsKind::Types(types) => {
                let xs = types
                    .iter()
                    .map(type_to_field_type)
                    .collect::<syn::Result<Vec<_>>>()?;
                Ok(quote! { (#(#xs),*) })
            }
        }
    }
}

pub fn type_to_field_type(ty: &Type) -> syn::Result<proc_macro2::TokenStream> {
    match ty {
        Type::Array(_) => Err(syn::Error::new_spanned(ty, "TODO: support arrays")),
        Type::Path(inner) => path_to_field_type(&inner.path),
        _ => Err(syn::Error::new_spanned(ty, "unsupported type")),
    }
}

fn path_to_field_type(path: &syn::Path) -> syn::Result<proc_macro2::TokenStream> {
    if path.segments.len() != 1 {
        return Err(syn::Error::new_spanned(
            path,
            "expected a single type path segment",
        ));
    }
    let seg = &path.segments[0];
    let ident = &seg.ident;
    let args = &seg.arguments;
    match args {
        syn::PathArguments::None => ident_to_field_type(ident),
        // Support for Vec<T> where T is a simple type
        syn::PathArguments::AngleBracketed(inner) if ident.eq("Vec") && inner.args.len() == 1 => {
            let inner_arg = &inner.args[0];
            if let syn::GenericArgument::Type(inner_ty) = inner_arg {
                let inner_type = type_to_field_type(inner_ty)?;
                Ok(quote! { FieldType::List(Box<#inner_type>) })
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
                Ok(quote! { FieldType::Optional(Box<#inner_type>) })
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

fn ident_to_field_type(ident: &Ident) -> syn::Result<proc_macro2::TokenStream> {
    match ident.to_string().as_str() {
        "u8" => Ok(quote! { FieldType::U8 }),
        "u16" => Ok(quote! { FieldType::U16 }),
        "u32" => Ok(quote! { FieldType::U32 }),
        "u64" => Ok(quote! { FieldType::U64 }),
        "i8" => Ok(quote! { FieldType::I8 }),
        "i16" => Ok(quote! { FieldType::I16 }),
        "i32" => Ok(quote! { FieldType::I32 }),
        "i64" => Ok(quote! { FieldType::I64 }),
        "bool" => Ok(quote! { FieldType::Bool }),
        "String" => Ok(quote! { FieldType::String }),
        "Bytes" => Ok(quote! { FieldType::Bytes }),
        "AccountId" => Ok(quote! { FieldType::AccountId }),
        _ => Err(syn::Error::new_spanned(ident, "unsupported type")),
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
