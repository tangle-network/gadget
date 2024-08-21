use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, Token};

// Defines custom keywords
mod kw {
    syn::custom_keyword!(threads);
}

/// `BenchmarkArgs` is a struct that holds the arguments for the `benchmark` macro.
pub(crate) struct BenchmarkArgs {
    /// The number of threads this benchmark should run with.
    ///
    /// `#[benchmark(threads = 4)]`
    threads: syn::LitInt,
}

pub(crate) fn benchmark_impl(args: &BenchmarkArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    let threads = &args.threads;
    let name = format_ident!("{}_benchmark", input.sig.ident.to_string());
    let block = &input.block;
    let expanded = quote! {
        #[doc(hidden)]
        fn #name() {
            let threads = #threads;
            // Start timers, watchers
            #block
            // Stop timers, watchers
            return;
        }
    };
    Ok(expanded.into())
}

impl Parse for BenchmarkArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut threads = None;
        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::threads) {
                let _ = input.parse::<kw::threads>()?;
                let _ = input.parse::<Token![=]>()?;
                threads = Some(input.parse()?);
            } else if lookahead.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            } else {
                return Err(lookahead.error());
            }
        }

        let threads =
            threads.ok_or_else(|| input.error("Missing `threads` argument in attribute"))?;

        Ok(Self { threads })
    }
}
