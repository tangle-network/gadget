use gadget_blueprint_proc_macro_core::{
    FieldType, ReportDefinition, ReportMetadata, ReportResultVerifier, ReportType,
};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::{collections::BTreeMap, time::Duration};
use syn::{
    parse::{Parse, ParseStream},
    Ident, ItemFn, LitInt, Token, Type,
};
use tokio::time::Instant;

use crate::job::{pascal_case, ResultsKind, Verifier};

mod kw {
    syn::custom_keyword!(id);
    syn::custom_keyword!(params);
    syn::custom_keyword!(result);
    syn::custom_keyword!(report_type);
    syn::custom_keyword!(job_id);
    syn::custom_keyword!(interval);
    syn::custom_keyword!(metric_thresholds);
    syn::custom_keyword!(verifier);
    syn::custom_keyword!(skip_codegen);
}
pub(crate) struct ReportArgs {
    id: LitInt,
    params: Vec<Ident>,
    result: ResultsKind,
    report_type: ReportType,
    job_id: Option<LitInt>,
    interval: Option<LitInt>,
    metric_thresholds: Option<Vec<(Ident, LitInt)>>,
    verifier: Verifier,
    skip_codegen: bool,
}

impl ReportArgs {
    fn params_to_field_types(
        &self,
        param_types: &BTreeMap<Ident, Type>,
    ) -> syn::Result<Vec<FieldType>> {
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
        Ok(params)
    }

    fn result_to_field_types(&self, result: &Type) -> syn::Result<Vec<FieldType>> {
        match &self.result {
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
}

pub(crate) fn report_impl(args: &ReportArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    // Extract function name and arguments
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let report_def_name = format_ident!("{}_REPORT_DEF", fn_name_string.to_ascii_uppercase());
    let report_id_name = format_ident!("{}_REPORT_ID", fn_name_string.to_ascii_uppercase());

    // Generate code based on report type
    let event_handler_gen = match args.report_type {
        ReportType::Job => generate_job_report_event_handler(args, input),
        ReportType::QoS => generate_qos_report_event_handler(args, input),
    };

    let report_def = ReportDefinition {
        metadata: ReportMetadata {
            name: fn_name_string.clone().into(),
            description: None, // This can be filled later during rustdoc gen
        },
        params: args.params_to_field_types(&param_types)?,
        result: args.result_to_field_types(result)?,
        verifier: match &args.verifier {
            Verifier::Evm(contract) => ReportResultVerifier::Evm(contract.clone()),
            Verifier::None => ReportResultVerifier::None,
        },
        report_type: args.report_type.clone(),
    };

    let report_def_str = serde_json::to_string(&report_def).map_err(|err| {
        syn::Error::new_spanned(
            input,
            format!("Failed to serialize report definition to json: {err}"),
        )
    })?;

    let gen = quote! {
        #[doc = "Report definition for the function "]
        #[doc = "[`"]
        #[doc = #fn_name_string]
        #[doc = "`]"]
        #[automatically_derived]
        #[doc(hidden)]
        pub const #report_def_name: &str = #report_def_str;

        #[doc = "Report ID for the function "]
        #[doc = "[`"]
        #[doc = #fn_name_string]
        #[doc = "`]"]
        #[automatically_derived]
        pub const #report_id_name: u8 = #args.id;

        #input

        #event_handler_gen

        // Add function to submit report on-chain
        fn submit_report(report: ReportData) {
            // Implement logic to craft and submit report on-chain
        }
    };

    Ok(gen.into())
}

fn generate_job_report_event_handler(
    args: &ReportArgs,
    input: &ItemFn,
) -> proc_macro2::TokenStream {
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let struct_name = format_ident!("{}JobReportEventHandler", pascal_case(&fn_name_string));
    let report_id = &args.id;
    let job_id = &args
        .job_id
        .as_ref()
        .expect("Job ID must be specified for job reports");

    quote! {
        pub struct #struct_name {
            pub service_id: u64,
            pub signer: gadget_sdk::tangle_subxt::subxt_signer::sr25519::Keypair,
        }

        #[automatically_derived]
        #[async_trait::async_trait]
        impl gadget_sdk::events_watcher::EventHandler<gadget_sdk::events_watcher::tangle::TangleConfig> for #struct_name {
            async fn can_handle_events(
                &self,
                events: gadget_sdk::tangle_subxt::subxt::events::Events<gadget_sdk::events_watcher::tangle::TangleConfig>,
            ) -> Result<bool, gadget_sdk::events_watcher::Error> {
                use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobResultSubmitted;

                let has_event = events.find::<JobResultSubmitted>().flatten().any(|event| {
                    event.service_id == self.service_id && event.job == #job_id
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
                use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobResultSubmitted;

                let job_results: Vec<_> = events
                    .find::<JobResultSubmitted>()
                    .flatten()
                    .filter(|event| event.service_id == self.service_id && event.job == #job_id)
                    .collect();

                for result in job_results {
                    let report_result = #fn_name(/* pass the appropriate parameters */);
                    if report_result != 0 {  // Assuming 0 means no issue
                        let report_data = ReportData {
                            service_id: self.service_id,
                            report_id: #report_id,
                            job_id: Some(#job_id),
                            block_number,
                            // Add other necessary fields
                        };
                        submit_report(client, &self.signer, report_data).await?;
                    }
                }

                Ok(())
            }
        }
    }
}

fn generate_qos_report_event_handler(
    args: &ReportArgs,
    input: &ItemFn,
) -> proc_macro2::TokenStream {
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let struct_name = format_ident!("{}QoSReportEventHandler", pascal_case(&fn_name_string));
    let report_id = &args.id;
    let interval = &args
        .interval
        .as_ref()
        .expect("Interval must be specified for QoS reports");

    quote! {
        pub struct #struct_name {
            pub service_id: u64,
            pub signer: gadget_sdk::tangle_subxt::subxt_signer::sr25519::Keypair,
        }

        #[automatically_derived]
        #[async_trait::async_trait]
        impl gadget_sdk::events_watcher::EventHandler<gadget_sdk::events_watcher::tangle::TangleConfig> for #struct_name {
            async fn can_handle_events(
                &self,
                _events: gadget_sdk::tangle_subxt::subxt::events::Events<gadget_sdk::events_watcher::tangle::TangleConfig>,
            ) -> Result<bool, gadget_sdk::events_watcher::Error> {
                // QoS reports are time-based, so we always return true
                Ok(true)
            }

            async fn handle_events(
                &self,
                client: gadget_sdk::tangle_subxt::subxt::OnlineClient<gadget_sdk::events_watcher::tangle::TangleConfig>,
                (_events, block_number): (
                    gadget_sdk::tangle_subxt::subxt::events::Events<gadget_sdk::events_watcher::tangle::TangleConfig>,
                    u64
                ),
            ) -> Result<(), gadget_sdk::events_watcher::Error> {
                use std::time::{Duration, Instant};
                use tokio::time::sleep;

                loop {
                    let start = Instant::now();

                    // Collect QoS metrics here
                    let qos_metrics = collect_qos_metrics().await?;
                    let report_result = #fn_name(
                        qos_metrics.uptime.as_secs() as f64,
                        qos_metrics.response_time.as_millis() as f64,
                        qos_metrics.error_rate,
                        qos_metrics.throughput as f64,
                        qos_metrics.memory_usage,
                        qos_metrics.cpu_usage
                    );
                    if !report_result {  // Assuming false means QoS breach
                        let report_data = ReportData {
                            service_id: self.service_id,
                            report_id: #report_id,
                            job_id: None,
                            block_number,
                            // Add other necessary fields
                        };
                        submit_report(client.clone(), &self.signer, report_data).await?;
                    }

                    let elapsed = start.elapsed();
                    if elapsed < Duration::from_secs(#interval) {
                        sleep(Duration::from_secs(#interval) - elapsed).await;
                    }
                }
            }
        }
    }
}
