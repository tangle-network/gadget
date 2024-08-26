use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, Token};

// Defines custom keywords
mod kw {
    syn::custom_keyword!(cores);
    syn::custom_keyword!(job_id);
}

/// `BenchmarkArgs` is a struct that holds the arguments for the `benchmark` macro.
pub(crate) struct BenchmarkArgs {
    /// The max number of cores this benchmark should run with.
    ///
    /// `#[benchmark(cores = 4)]`
    cores: syn::LitInt,
    /// The job identifier for the benchmark.
    ///
    /// `#[benchmark(job_id = 1)]`
    job_id: syn::LitInt,
}

pub(crate) fn benchmark_impl(args: &BenchmarkArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    let cores = &args.cores;
    let job_id = &args.job_id;
    let original_name = &input.sig.ident;
    let name = format_ident!("{}_benchmark", original_name);
    let block = &input.block;
    let expanded = quote! {
        #[doc(hidden)]
        pub fn #name() {
            let cores: usize = #cores;
            let rt = gadget_sdk::benchmark::tokio::runtime::Builder::new_multi_thread()
                .worker_threads(cores)
                .max_blocking_threads(cores)
                .enable_all()
                .build()
                .unwrap();
            let _guard = rt.enter();
            let b = gadget_sdk::benchmark::Bencher::new(cores, gadget_sdk::benchmark::TokioRuntime);
            b.block_on(async move { #block });
            let summary = b.stop(stringify!(#original_name), #job_id);
            eprintln!("{}", summary);
            return;
        }
    };
    Ok(expanded.into())
}

impl Parse for BenchmarkArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut cores = None;
        let mut job_id = None;
        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::cores) {
                let _ = input.parse::<kw::cores>()?;
                let _ = input.parse::<Token![=]>()?;
                cores = Some(input.parse()?);
            } else if lookahead.peek(kw::job_id) {
                let _ = input.parse::<kw::job_id>()?;
                let _ = input.parse::<Token![=]>()?;
                job_id = Some(input.parse()?);
            } else if lookahead.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            } else {
                return Err(lookahead.error());
            }
        }

        let cores = cores.ok_or_else(|| input.error("Missing `cores` argument in attribute"))?;

        let job_id = job_id.ok_or_else(|| input.error("Missing `job_id` argument in attribute"))?;

        Ok(Self { cores, job_id })
    }
}
