use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    Ident, ItemFn, LitInt, Token,
};

use crate::job::{Params, Results, ResultsKind, Verifier};

// Defines custom keywords
mod kw {
    syn::custom_keyword!(id);
    syn::custom_keyword!(params);
    syn::custom_keyword!(result);
    syn::custom_keyword!(verifier);
    syn::custom_keyword!(evm);
    syn::custom_keyword!(skip_codegen);
}

pub(crate) struct ReportArgs {
    id: LitInt,
    /// List of parameters for the report, in order.
    /// `#[report(params(a, b, c))]`
    params: Vec<Ident>,
    /// List of return types for the report, could be infered from the function return type.
    /// `#[report(result(u32, u64))]`
    /// `#[report(result(_))]`
    result: ResultsKind,
    /// Optional: Verifier for the report result, currently only supports EVM verifier.
    /// `#[report(verifier(evm = "MyVerifierContract"))]`
    verifier: Verifier,
    /// Optional: Skip code generation for this report.
    /// `#[report(skip_codegen)]`
    /// this is useful if the developer want to impl a custom event handler
    /// for this job.
    skip_codegen: bool,
}

impl Parse for ReportArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut params = Vec::new();
        let mut result = None;
        let mut id = None;
        let mut verifier = Verifier::None;
        let mut skip_codegen = false;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::id) {
                let _ = input.parse::<kw::id>()?;
                let _ = input.parse::<Token![=]>()?;
                id = Some(input.parse()?);
            } else if lookahead.peek(kw::params) {
                let Params(p) = input.parse()?;
                params = p;
            } else if lookahead.peek(kw::result) {
                let Results(r) = input.parse()?;
                result = Some(r);
            } else if lookahead.peek(kw::verifier) {
                verifier = input.parse()?;
            } else if lookahead.peek(kw::skip_codegen) {
                let _ = input.parse::<kw::skip_codegen>()?;
                skip_codegen = true;
            } else if lookahead.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            } else {
                return Err(lookahead.error());
            }
        }

        let id = id.ok_or_else(|| input.error("Missing `id` argument in attribute"))?;

        if params.is_empty() {
            return Err(input.error("Missing `params` argument in attribute"));
        }

        let result = result.ok_or_else(|| input.error("Missing 'result' argument in attribute"))?;

        if let ResultsKind::Types(ref r) = result {
            if r.is_empty() {
                return Err(input.error("Expected at least one parameter for the `result` attribute, or `_` to infer the type"));
            }
        }

        Ok(ReportArgs {
            id,
            params,
            result,
            verifier,
            skip_codegen,
        })
    }
}

pub(crate) fn report_impl(args: &ReportArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let report_fn_name = format_ident!("{}_report", fn_name_string);
    let job_id = &args.id;
    let event_handler_name = format_ident!("{}EventHandler", fn_name_string);

    // Extract the parameters from the function signature, excluding the first one (Result)
    let inputs = input.sig.inputs.iter().skip(1);

    let gen = quote! {
        #[doc = "Report function for the job with ID "]
        #[doc = #job_id]
        #[automatically_derived]
        pub fn #report_fn_name(#(#inputs),*) -> u8 {
            // Implement your logic here to calculate the percentage of stake to be slashed
            // For now, we return a dummy value
            0
        }

        #[automatically_derived]
        pub struct #event_handler_name;

        #[automatically_derived]
        #[async_trait::async_trait]
        impl gadget_sdk::events_watcher::EventHandler<gadget_sdk::events_watcher::tangle::TangleConfig> for #event_handler_name {
            async fn can_handle_events(
                &self,
                events: gadget_sdk::tangle_subxt::subxt::events::Events<gadget_sdk::events_watcher::tangle::TangleConfig>,
            ) -> Result<bool, gadget_sdk::events_watcher::Error> {
                use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;

                let has_event = events.find::<JobCalled>().flatten().any(|event| {
                    event.job == #job_id
                });

                Ok(has_event)
            }

            async fn handle_events(
                &self,
                client: gadget_sdk::tangle_subxt::subxt::OnlineClient<gadget_sdk::events_watcher::tangle::TangleConfig>,
                (events, block_number): (
                    gadget_sdk::tangle_subxt::subxt::events::Events<gadget_sdk::events_watcher::tangle::TangleConfig>,
                    u64
                ),
            ) -> Result<(), gadget_sdk::events_watcher::Error> {
                use gadget_sdk::tangle_subxt::{
                    subxt,
                    tangle_testnet_runtime::api::{
                        self as TangleApi,
                        runtime_types::{
                            bounded_collections::bounded_vec::BoundedVec, tangle_primitives::services::field::Field,
                        },
                        services::events::{JobCalled, JobResultSubmitted},
                    },
                };
                let job_events: Vec<_> = events
                    .find::<JobCalled>()
                    .flatten()
                    .filter(|event| {
                        event.job == #job_id
                    })
                    .collect();
                for call in job_events {
                    tracing::info!("Handling JobCalled Events: #{block_number}",);

                    let mut args_iter = call.args.into_iter();
                    // Extract parameters from the event and call the report function
                    let value = args_iter.next().unwrap(); // Extract the actual value
                    let slash_percentage = #report_fn_name(value);
                    tracing::info!("Slash percentage: {}", slash_percentage);
                }
                Ok(())
            }
        }

        #input
    };

    Ok(gen.into())
}
