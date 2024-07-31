use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::{Parse, ParseStream}, ItemFn, LitInt, Token};

mod kw {
    syn::custom_keyword!(id);
}

pub(crate) struct ReportArgs {
    id: LitInt,
}

impl Parse for ReportArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<kw::id>()?;
        let _ = input.parse::<Token![=]>()?;
        let id = input.parse()?;
        Ok(Self { id })
    }
}

pub(crate) fn report_impl(args: &ReportArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let report_fn_name = format_ident!("{}_report", fn_name_string);
    let job_id = &args.id;

    let gen = quote! {
        #[doc = "Report function for the job with ID "]
        #[doc = #job_id]
        #[automatically_derived]
        pub fn #report_fn_name(#(#input.sig.inputs)*) -> u8 {
            // Implement your logic here to calculate the percentage of stake to be slashed
            // For now, we return a dummy value
            0
        }

        #input
    };

    Ok(gen.into())
}