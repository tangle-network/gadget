use std::collections::{BTreeMap, HashSet};

use proc_macro::TokenStream;
use quote::{format_ident, quote};
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

    let syn::ReturnType::Type(_, _result) = &input.sig.output else {
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
    let result_type = args.result_to_token_stream();
    // Generate the struct
    let gen = quote! {
        struct #struct_name {
            params: #params_type,
            result: #result_type,
        }

        #input
    };

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
    result: Vec<Type>,
}

impl Parse for JobArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut params = Vec::new();
        let mut result = Vec::new();

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::params) {
                let Params(p) = input.parse()?;
                params = p;
            } else if lookahead.peek(kw::result) {
                let Results(r) = input.parse()?;
                result = r;
            } else if lookahead.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            } else {
                return Err(lookahead.error());
            }
        }

        if params.is_empty() {
            return Err(input.error("Missing 'params' argument in attribute"));
        }

        if result.is_empty() {
            return Err(input.error("Missing 'result' argument in attribute"));
        }

        Ok(JobArgs { params, result })
    }
}

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

struct Results(Vec<Type>);

impl Parse for Results {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let _ = input.parse::<kw::result>();
        let content;
        let _ = syn::parenthesized!(content in input);
        let names = content.parse_terminated(Type::parse, Token![,])?;
        let mut items = Vec::new();
        for name in names {
            items.push(name);
        }
        Ok(Self(items))
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
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(quote! { (#(#params),*) })
    }

    fn result_to_token_stream(&self) -> proc_macro2::TokenStream {
        let result = self.result.iter().collect::<Vec<_>>();
        quote! { (#(#result),*) }
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
