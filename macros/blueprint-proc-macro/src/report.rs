use crate::utils::{pascal_case, type_to_field_type};
use gadget_blueprint_proc_macro_core::{
    FieldType, ReportDefinition, ReportMetadata, ReportResultVerifier, ReportType,
};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeMap;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    Ident, ItemFn, LitInt, LitStr, Token, Type,
};

mod kw {
    syn::custom_keyword!(params);
    syn::custom_keyword!(result);
    syn::custom_keyword!(report_type);
    syn::custom_keyword!(job_id);
    syn::custom_keyword!(interval);
    syn::custom_keyword!(metric_thresholds);
    syn::custom_keyword!(verifier);
    syn::custom_keyword!(evm);
    syn::custom_keyword!(skip_codegen);
}

/// Represents the arguments passed to the `report` attribute macro.
pub(crate) struct ReportArgs {
    params: Vec<Ident>,
    result: ResultsKind,
    report_type: ReportType,
    job_id: Option<LitInt>,
    interval: Option<LitInt>,
    metric_thresholds: Option<Vec<(Ident, LitInt)>>,
    verifier: Verifier,
    skip_codegen: bool,
}

/// Parses the arguments provided to the `report` attribute macro.
impl Parse for ReportArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut params = Vec::new();
        let mut result = None;
        let mut report_type = None;
        let mut job_id = None;
        let mut interval = None;
        let mut metric_thresholds = None;
        let mut verifier = Verifier::None;
        let mut skip_codegen = false;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::params) {
                let _ = input.parse::<kw::params>()?;
                let content;
                parenthesized!(content in input);
                params = content
                    .parse_terminated(Ident::parse, Token![,])?
                    .into_iter()
                    .collect();
            } else if lookahead.peek(kw::result) {
                let Results(r) = input.parse()?;
                result = Some(r);
            } else if lookahead.peek(kw::report_type) {
                let _ = input.parse::<kw::report_type>()?;
                let _ = input.parse::<Token![=]>()?;
                let type_str: LitStr = input.parse()?;
                report_type = Some(match type_str.value().as_str() {
                    "job" => ReportType::Job,
                    "qos" => ReportType::QoS,
                    _ => {
                        return Err(syn::Error::new(
                            type_str.span(),
                            "Invalid report type. Expected 'job' or 'qos'.",
                        ))
                    }
                });
            } else if lookahead.peek(kw::job_id) {
                let _ = input.parse::<kw::job_id>()?;
                let _ = input.parse::<Token![=]>()?;
                job_id = Some(input.parse::<LitInt>()?);
            } else if lookahead.peek(kw::interval) {
                let _ = input.parse::<kw::interval>()?;
                let _ = input.parse::<Token![=]>()?;
                interval = Some(input.parse::<LitInt>()?);
            } else if lookahead.peek(kw::metric_thresholds) {
                let _ = input.parse::<kw::metric_thresholds>()?;
                let content;
                parenthesized!(content in input);
                let thresholds = content.parse_terminated(
                    |input| {
                        let name = input.parse::<Ident>()?;
                        let _ = input.parse::<Token![=]>()?;
                        let value = input.parse::<LitInt>()?;
                        Ok((name, value))
                    },
                    Token![,],
                )?;
                metric_thresholds = Some(thresholds.into_iter().collect());
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

        // Validation logic
        let report_type = report_type
            .ok_or_else(|| input.error("Missing `type` argument in report attribute"))?;

        if params.is_empty() {
            return Err(input.error("Missing `params` argument in report attribute"));
        }

        let result =
            result.ok_or_else(|| input.error("Missing `result` argument in report attribute"))?;

        match report_type {
            ReportType::Job => {
                if job_id.is_none() {
                    return Err(input.error("Missing `job_id` for job report"));
                }
            }
            ReportType::QoS => {
                if interval.is_none() {
                    return Err(input.error("Missing `interval` for QoS report"));
                }
                if job_id.is_some() {
                    return Err(input.error("Unexpected `job_id` for QoS report"));
                }
            }
        }

        Ok(ReportArgs {
            params,
            result,
            report_type,
            job_id,
            interval,
            metric_thresholds,
            verifier,
            skip_codegen,
        })
    }
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

/// Implements the core functionality of the `report` attribute macro.
///
/// This function generates the necessary code for both job reports and `QoS` reports,
/// including the report definition and the appropriate event handler.
///
/// # Arguments
///
/// * `args` - The parsed arguments from the `report` attribute macro.
/// * `input` - The function item that the `report` attribute is attached to.
///
/// # Returns
///
/// Returns a `Result` containing the generated `TokenStream` or a `syn::Error`.
pub(crate) fn report_impl(args: &ReportArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let report_def_name = format_ident!("{}_REPORT_DEF", fn_name_string.to_ascii_uppercase());

    let mut param_types = BTreeMap::new();
    for input in &input.sig.inputs {
        if let syn::FnArg::Typed(arg) = input {
            if let syn::Pat::Ident(pat_ident) = &*arg.pat {
                let ident = &pat_ident.ident;
                let ty = &*arg.ty;
                let added = param_types.insert(ident.clone(), ty.clone());
                if added.is_some() {
                    return Err(syn::Error::new_spanned(ident, "Duplicate parameter name"));
                }
            }
        }
    }

    let params_type = args.params_to_field_types(&param_types)?;

    let syn::ReturnType::Type(_, result) = &input.sig.output else {
        return Err(syn::Error::new_spanned(
            &input.sig.output,
            "Function must have a return type",
        ));
    };

    let result_type = args.result_to_field_types(result)?;

    let report_def = ReportDefinition {
        metadata: ReportMetadata {
            name: fn_name_string.clone().into(),
            description: None, // TODO: Add support for description
        },
        params: params_type.clone(),
        result: result_type.clone(),
        report_type: args.report_type.clone(),
        job_id: args.job_id.as_ref().and_then(|lit| lit.base10_parse().ok()),
        interval: args
            .interval
            .as_ref()
            .and_then(|lit| lit.base10_parse().ok()),
        metric_thresholds: args.metric_thresholds.as_ref().map(|thresholds| {
            thresholds
                .iter()
                .filter_map(|(ident, lit_int)| {
                    lit_int
                        .base10_parse::<u64>()
                        .ok()
                        .map(|value| (ident.to_string(), value))
                })
                .collect()
        }),
        verifier: match &args.verifier {
            Verifier::Evm(contract) => ReportResultVerifier::Evm(contract.clone()),
            Verifier::None => ReportResultVerifier::None,
        },
    };

    let report_def_str = serde_json::to_string(&report_def).map_err(|err| {
        syn::Error::new_spanned(
            input,
            format!("Failed to serialize report definition to json: {err}"),
        )
    })?;

    let event_handler_gen = if args.skip_codegen {
        proc_macro2::TokenStream::new()
    } else {
        match args.report_type {
            ReportType::Job => {
                generate_job_report_event_handler(args, input, &params_type, &result_type)
            }
            ReportType::QoS => {
                generate_qos_report_event_handler(args, input, &params_type, &result_type)
            }
        }
    };

    let gen = quote! {
        #[doc = "Report definition for the function "]
        #[doc = #fn_name_string]
        pub const #report_def_name: &str = #report_def_str;

        #input

        #event_handler_gen
    };

    println!("Generated code:\n{gen}");

    Ok(gen.into())
}

/// Generates an event handler for job reports.
///
/// This function creates a struct that listens for job result submissions
/// and triggers the report function when necessary.
///
/// # Arguments
///
/// * `args` - The parsed arguments from the `report` attribute macro.
/// * `input` - The function item that the `report` attribute is attached to.
/// * `param_types` - A map of parameter names to their types.
/// * `params` - The list of parameter field types.
/// * `result` - The list of result field types.
///
/// # Returns
///
/// Returns a `TokenStream` containing the generated event handler code.
fn generate_job_report_event_handler(
    args: &ReportArgs,
    input: &ItemFn,
    params: &[FieldType],
    result: &[FieldType],
) -> proc_macro2::TokenStream {
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let struct_name = format_ident!("{}JobReportEventHandler", pascal_case(&fn_name_string));
    let job_id = args
        .job_id
        .as_ref()
        .expect("Job ID must be present for job reports");

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
                _client: gadget_sdk::tangle_subxt::subxt::OnlineClient<gadget_sdk::events_watcher::tangle::TangleConfig>,
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

                for job_result in job_results {
                    // TODO: Implement parameter extraction and report function call
                    // This part will depend on the specific structure of your JobResultSubmitted event
                    // and how you want to handle the report function call
                }

                Ok(())
            }
        }
    }
}

/// Generates an event handler for `QoS` reports.
///
/// This function creates a struct that periodically collects `QoS` metrics
/// and triggers the report function at specified intervals.
///
/// # Arguments
///
/// * `args` - The parsed arguments from the `report` attribute macro.
/// * `input` - The function item that the `report` attribute is attached to.
/// * `param_types` - A map of parameter names to their types.
/// * `params` - The list of parameter field types.
/// * `result` - The list of result field types.
///
/// # Returns
///
/// Returns a `TokenStream` containing the generated event handler code.
fn generate_qos_report_event_handler(
    args: &ReportArgs,
    input: &ItemFn,
    params: &[FieldType],
    result: &[FieldType],
) -> proc_macro2::TokenStream {
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let struct_name = format_ident!("{}QoSReportEventHandler", pascal_case(&fn_name_string));
    let interval = args
        .interval
        .as_ref()
        .expect("Interval must be present for QoS reports");

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
                Ok(true)
            }

            async fn handle_events(
                &self,
                _client: gadget_sdk::tangle_subxt::subxt::OnlineClient<gadget_sdk::events_watcher::tangle::TangleConfig>,
                (_events, _block_number): (
                    gadget_sdk::tangle_subxt::subxt::events::Events<gadget_sdk::events_watcher::tangle::TangleConfig>,
                    u64
                ),
            ) -> Result<(), gadget_sdk::events_watcher::Error> {
                use std::time::Duration;
                use gadget_sdk::slashing::reports::{QoSReporter, DefaultQoSReporter};


                let mut reporter = DefaultQoSReporter { service_id: self.service_id };
                let interval = Duration::from_secs(#interval);
                let mut next_check = std::time::Instant::now();

                loop {
                    if std::time::Instant::now() >= next_check {
                        let metrics = reporter.collect_metrics().await
                            .map_err(|e| gadget_sdk::events_watcher::Error::Handler(e))?;

                        let report_result = reporter.report(&metrics).await
                            .map_err(|e| gadget_sdk::events_watcher::Error::Handler(e))?;

                        next_check = std::time::Instant::now() + interval;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
}

pub enum ResultsKind {
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

#[derive(Debug)]
enum Verifier {
    None,
    // #[job(verifier(evm = "MyVerifierContract"))]
    Evm(String),
}

impl Parse for Verifier {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let _ = input.parse::<kw::verifier>()?;
        let content;
        let _ = syn::parenthesized!(content in input);
        let lookahead = content.lookahead1();
        // parse `(evm = "MyVerifierContract")`
        if lookahead.peek(kw::evm) {
            let _ = content.parse::<kw::evm>()?;
            let _ = content.parse::<Token![=]>()?;
            let contract = content.parse::<LitStr>()?;
            Ok(Verifier::Evm(contract.value()))
        } else {
            Ok(Verifier::None)
        }
    }
}

#[doc = "Report definition for the function "]
#[doc = "report_keygen"]
pub
const REPORT_KEYGEN_REPORT_DEF : & str =
"{\"metadata\":{\"name\":\"report_keygen\",\"description\":null},\"params\":[\"Uint16\",\"Uint16\",{\"List\":\"Bytes\"}],\"result\":[\"Uint32\"],\"report_type\":\"job\",\"job_id\":0,\"verifier\":{\"evm\":\"KeygenContract\"}}";
#[doc = " Report function for the keygen job."]
fn report_keygen(n: u16, t: u16, msgs: Vec<Vec<u8>>) -> u32 {
    let _ = (n, t, msgs);
    0
}
pub struct ReportKeygenJobReportEventHandler {
    pub service_id: u64,
    pub signer: gadget_sdk::tangle_subxt::subxt_signer::sr25519::Keypair,
}
#[automatically_derived]
#[async_trait::async_trait]
impl gadget_sdk::events_watcher::EventHandler<gadget_sdk::events_watcher::tangle::TangleConfig>
    for ReportKeygenJobReportEventHandler
{
    async fn can_handle_events(
        &self,
        events: gadget_sdk::tangle_subxt::subxt::events::Events<
            gadget_sdk::events_watcher::tangle::TangleConfig,
        >,
    ) -> Result<bool, gadget_sdk::events_watcher::Error> {
        use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobResultSubmitted;
        let has_event = events
            .find::<JobResultSubmitted>()
            .flatten()
            .any(|event| event.service_id == self.service_id && event.job == 0);
        Ok(has_event)
    }
    async fn handle_events(
        &self,
        _client: gadget_sdk::tangle_subxt::subxt::OnlineClient<
            gadget_sdk::events_watcher::tangle::TangleConfig,
        >,
        (events, block_number): (
            gadget_sdk::tangle_subxt::subxt::events::Events<
                gadget_sdk::events_watcher::tangle::TangleConfig,
            >,
            u64,
        ),
    ) -> Result<(), gadget_sdk::events_watcher::Error> {
        use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobResultSubmitted;
        let job_results: Vec<_> = events
            .find::<JobResultSubmitted>()
            .flatten()
            .filter(|event| event.service_id == self.service_id && event.job == 0)
            .collect();
        for job_result in job_results {}
        Ok(())
    }
}

#[doc = "Report definition for the function "]
#[doc = "report_service_health"]
pub const REPORT_SERVICE_HEALTH_REPORT_DEF :
& str =
"{\"metadata\":{\"name\":\"report_service_health\",\"description\":null},\"params\":[\"Float64\",\"Uint64\",\"Float64\"],\"result\":[\"Bytes\"],\"report_type\":\"qos\",\"interval\":3600,\"metric_thresholds\":[[\"uptime\",99],[\"response_time\",1000],[\"error_rate\",5]],\"verifier\":\"none\"}";
fn report_service_health(uptime: f64, response_time: u64, error_rate: f64) -> Vec<u8> {
    let mut issues = Vec::new();
    if uptime < 99.0 {
        issues.push(b"Low uptime".to_vec());
    }
    if response_time > 1000 {
        issues.push(b"High response time".to_vec());
    }
    if error_rate > 5.0 {
        issues.push(b"High error rate".to_vec());
    }
    issues.concat()
}
pub struct ReportServiceHealthQoSReportEventHandler {
    pub service_id: u64,
    pub signer: gadget_sdk::tangle_subxt::subxt_signer::sr25519::Keypair,
}
#[automatically_derived]
#[async_trait::async_trait]
impl gadget_sdk::events_watcher::EventHandler<gadget_sdk::events_watcher::tangle::TangleConfig>
    for ReportServiceHealthQoSReportEventHandler
{
    async fn can_handle_events(
        &self,
        _events: gadget_sdk::tangle_subxt::subxt::events::Events<
            gadget_sdk::events_watcher::tangle::TangleConfig,
        >,
    ) -> Result<bool, gadget_sdk::events_watcher::Error> {
        Ok(true)
    }
    async fn handle_events(
        &self,
        _client: gadget_sdk::tangle_subxt::subxt::OnlineClient<
            gadget_sdk::events_watcher::tangle::TangleConfig,
        >,
        (_events, _block_number): (
            gadget_sdk::tangle_subxt::subxt::events::Events<
                gadget_sdk::events_watcher::tangle::TangleConfig,
            >,
            u64,
        ),
    ) -> Result<(), gadget_sdk::events_watcher::Error> {
        use gadget_sdk::slashing::reports::{DefaultQoSReporter, QoSReporter};
        use std::time::Duration;
        let mut reporter = DefaultQoSReporter {
            service_id: self.service_id,
        };
        let interval = Duration::from_secs(3600);
        let mut next_check = std::time::Instant::now();
        loop {
            if std::time::Instant::now() >= next_check {
                let metrics = reporter
                    .collect_metrics()
                    .await
                    .map_err(|e| gadget_sdk::events_watcher::Error::Handler(e))?;
                let report_result = reporter
                    .report(&metrics)
                    .await
                    .map_err(|e| gadget_sdk::events_watcher::Error::Handler(e))?;
                next_check = std::time::Instant::now() + interval;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
