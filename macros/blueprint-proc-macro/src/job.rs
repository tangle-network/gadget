use crate::eigenlayer::event_listener::generate_eigenlayer_event_handler;
use crate::shared::{pascal_case, type_to_field_type};
use crate::tangle::event_listener::generate_tangle_event_handler;
use gadget_blueprint_proc_macro_core::{FieldType, JobDefinition, JobMetadata, JobResultVerifier};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::{BTreeMap, HashSet};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, LitInt, LitStr, Token, Type};

// Defines custom keywords
mod kw {
    syn::custom_keyword!(id);
    syn::custom_keyword!(params);
    syn::custom_keyword!(result);
    syn::custom_keyword!(verifier);
    syn::custom_keyword!(evm);
    syn::custom_keyword!(event_handler);
    syn::custom_keyword!(protocol);
    syn::custom_keyword!(instance);
    syn::custom_keyword!(event);
    syn::custom_keyword!(event_converter);
    syn::custom_keyword!(callback);
    syn::custom_keyword!(skip_codegen);
}

pub(crate) fn job_impl(args: &JobArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    // Extract function name and arguments
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let job_def_name = format_ident!("{}_JOB_DEF", fn_name_string.to_ascii_uppercase());
    let job_id_name = format_ident!("{}_JOB_ID", fn_name_string.to_ascii_uppercase());

    let syn::ReturnType::Type(_, result) = &input.sig.output else {
        return Err(syn::Error::new_spanned(
            &input.sig.output,
            "Function must have a return type of Result<T, E> where T is a tuple of the result fields",
        ));
    };

    // check that the function has a return type of Result<T, E>
    match **result {
        Type::Path(ref path) => {
            let seg = path.path.segments.last().unwrap();
            if seg.ident != "Result" {
                return Err(syn::Error::new_spanned(
                    result,
                    "Function must have a return type of Result<T, E> where T is a tuple of the result fields",
                ));
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                result,
                "Function must have a return type of Result<T, E> where T is a tuple of the result fields",
            ));
        }
    }

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

    let job_id = &args.id;
    let params_type = args.params_to_field_types(&param_types)?;
    let result_type = args.result_to_field_types(result)?;

    let event_handler_gen = if args.skip_codegen {
        proc_macro2::TokenStream::default()
    } else {
        // Generate the Job Handler.
        generate_event_handler_for(input, args, &param_types, &params_type, &result_type)
    };

    // Extract params and result types from args
    let job_def = JobDefinition {
        metadata: JobMetadata {
            name: fn_name_string.clone().into(),
            // filled later on during the rustdoc gen.
            description: None,
        },
        params: params_type,
        result: result_type,
        verifier: match &args.verifier {
            Verifier::Evm(contract) => JobResultVerifier::Evm(contract.clone()),
            Verifier::None => JobResultVerifier::None,
        },
    };

    let job_def_str = serde_json::to_string(&job_def).map_err(|err| {
        syn::Error::new_spanned(
            input,
            format!("Failed to serialize job definition to json: {err}"),
        )
    })?;

    let gen = quote! {
        #[doc = "Job definition for the function "]
        #[doc = "[`"]
        #[doc = #fn_name_string]
        #[doc = "`]"]
        #[automatically_derived]
        #[doc(hidden)]
        pub const #job_def_name: &str = #job_def_str;

        #[doc = "Job ID for the function "]
        #[doc = "[`"]
        #[doc = #fn_name_string]
        #[doc = "`]"]
        #[automatically_derived]
        pub const #job_id_name: u8 = #job_id;

        #input

        #event_handler_gen
    };

    Ok(gen.into())
}

#[allow(clippy::too_many_lines)]
pub fn generate_event_handler_for(
    f: &ItemFn,
    job_args: &JobArgs,
    param_types: &BTreeMap<Ident, Type>,
    params: &[FieldType],
    result: &[FieldType],
) -> proc_macro2::TokenStream {
    let fn_name = &f.sig.ident;
    let fn_name_string = fn_name.to_string();
    let struct_name = format_ident!("{}EventHandler", pascal_case(&fn_name_string));
    let job_id = &job_args.id;
    let event_handler = &job_args.event_handler;

    // Get all the params names inside the param_types map
    // and not in the params list to be added to the event handler.
    let x = param_types.keys().collect::<HashSet<_>>();
    let y = job_args.params.iter().collect::<HashSet<_>>();
    let diff = x.difference(&y).collect::<Vec<_>>();
    let additional_params = diff
        .iter()
        .map(|ident| {
            let mut ty = param_types[**ident].clone();
            // remove the reference from the type and use the inner type
            if let Type::Reference(r) = ty {
                ty = *r.elem;
            }
            quote! {
                pub #ident: #ty,
            }
        })
        .collect::<Vec<_>>();

    let additional_params_in_call = diff
        .iter()
        .map(|ident| {
            let ty = &param_types[**ident];
            let (is_ref, is_ref_mut) = match ty {
                Type::Reference(r) => (true, r.mutability.is_some()),
                _ => (false, false),
            };
            if is_ref && is_ref_mut {
                quote! { &mut self.#ident, }
            } else if is_ref {
                quote! { &self.#ident, }
            } else {
                quote! { self.#ident, }
            }
        })
        .collect::<Vec<_>>();

    let params_tokens = params
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let ident = format_ident!("param{i}");
            let index = syn::Index::from(i);
            match event_handler {
                EventHandlerArgs::Eigenlayer { .. } => {
                    quote! {
                        let #ident = inputs.#index;
                    }
                }
                EventHandlerArgs::Tangle => crate::tangle::field_type_to_param_token(&ident, t),
            }
        })
        .collect::<Vec<_>>();

    let fn_call_params = params
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let ident = format_ident!("param{i}");
            quote! {
                #ident,
            }
        })
        .collect::<Vec<_>>();
    let fn_call = if f.sig.asyncness.is_some() {
        quote! {
            let job_result = match #fn_name(
                #(#additional_params_in_call)*
                #(#fn_call_params)*
            ).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Error in job: {e}");
                    let error = gadget_sdk::events_watcher::Error::Handler(Box::new(e));
                    return Err(error);
                }
            };
        }
    } else {
        quote! {
            let job_result = match #fn_name(
                #(#additional_params_in_call)*
                #(#fn_call_params)*
            ) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Error in job: {e}");
                    let error = gadget_sdk::events_watcher::Error::Handler(Box::new(e));
                    return Err(error);
                }
            };
        }
    };

    let result_tokens = if result.len() == 1 {
        let ident = format_ident!("job_result");
        match event_handler {
            EventHandlerArgs::Eigenlayer { .. } => {
                vec![quote! { let #ident = job_result; }]
            }
            EventHandlerArgs::Tangle => {
                vec![crate::tangle::field_type_to_result_token(
                    &ident, &result[0],
                )]
            }
        }
    } else {
        result
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let ident = format_ident!("result_{i}");
                match event_handler {
                    EventHandlerArgs::Eigenlayer { .. } => {
                        quote! {
                            let #ident = job_result[#i];
                        }
                    }
                    EventHandlerArgs::Tangle => {
                        let s = crate::tangle::field_type_to_result_token(&ident, t);
                        quote! {
                            let #ident = job_result[#i];
                            #s
                        }
                    }
                }
            })
            .collect::<Vec<_>>()
    };

    if event_handler.is_eigenlayer() {
        generate_eigenlayer_event_handler(
            &fn_name_string,
            &struct_name,
            event_handler,
            &params_tokens,
            &additional_params,
            &fn_call,
        )
    } else {
        generate_tangle_event_handler(
            &fn_name_string,
            &struct_name,
            job_id,
            &params_tokens,
            &result_tokens,
            &additional_params,
            &fn_call,
        )
    }
}

/// `JobArgs` type to handle parsing of attributes
pub(crate) struct JobArgs {
    /// Unique identifier for the job in the blueprint
    /// `#[job(id = 1)]`
    id: LitInt,
    /// List of parameters for the job, in order.
    /// `#[job(params(a, b, c))]`
    params: Vec<Ident>,
    /// List of return types for the job, could be infered from the function return type.
    /// `#[job(result(u32, u64))]`
    /// `#[job(result(_))]`
    result: ResultsKind,
    /// Optional: Verifier for the job result, currently only supports EVM verifier.
    /// `#[job(verifier(evm = "MyVerifierContract"))]`
    verifier: Verifier,
    /// Optional: Event handler type for the job.
    /// `#[job(event_handler = "tangle")]`
    event_handler: EventHandlerArgs,
    /// Optional: Skip code generation for this job.
    /// `#[job(skip_codegen)]`
    /// this is useful if the developer want to impl a custom event handler
    /// for this job.
    skip_codegen: bool,
}

impl Parse for JobArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut params = Vec::new();
        let mut result = None;
        let mut id = None;
        let mut verifier = Verifier::None;
        let mut event_handler = EventHandlerArgs::Tangle;
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
            } else if lookahead.peek(kw::event_handler) {
                event_handler = input.parse()?;
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

        Ok(JobArgs {
            id,
            params,
            result,
            verifier,
            event_handler,
            skip_codegen,
        })
    }
}

#[derive(Debug)]
pub struct Params(pub Vec<Ident>);

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

impl JobArgs {
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
    /// #[job(verifier(evm = "`MyVerifierContract`"))]
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

pub(crate) enum EventHandlerArgs {
    Tangle,
    Eigenlayer {
        instance: Option<Ident>,
        event: Option<Type>,
        event_converter: Option<Type>,
        callback: Option<Type>,
    },
}

impl EventHandlerArgs {
    pub fn is_eigenlayer(&self) -> bool {
        matches!(self, Self::Eigenlayer { .. })
    }

    pub fn instance(&self) -> Option<Ident> {
        match self {
            Self::Eigenlayer { instance, .. } => instance.clone(),
            Self::Tangle => None,
        }
    }

    pub fn event(&self) -> Option<Type> {
        match self {
            Self::Eigenlayer { event, .. } => event.clone(),
            Self::Tangle => None,
        }
    }

    pub fn event_converter(&self) -> Option<Type> {
        match self {
            Self::Eigenlayer {
                event_converter, ..
            } => event_converter.clone(),
            Self::Tangle => None,
        }
    }

    pub fn callback(&self) -> Option<Type> {
        match self {
            Self::Eigenlayer { callback, .. } => callback.clone(),
            Self::Tangle => None,
        }
    }
}

impl Parse for EventHandlerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<kw::event_handler>()?;
        let content;
        syn::parenthesized!(content in input);

        let protocol = if content.peek(kw::protocol) {
            let _ = content.parse::<kw::protocol>()?;
            let _ = content.parse::<Token![=]>()?;
            content.parse::<LitStr>()?.value()
        } else {
            "tangle".to_string()
        };

        match protocol.as_str() {
            "tangle" => Ok(EventHandlerArgs::Tangle),
            "eigenlayer" => {
                let mut instance = None;
                let mut event = None;
                let mut event_converter = None;
                let mut callback = None;

                while !content.is_empty() {
                    if content.peek(kw::instance) {
                        let _ = content.parse::<kw::instance>()?;
                        let _ = content.parse::<Token![=]>()?;
                        instance = Some(content.parse::<Ident>()?);
                    } else if content.peek(kw::event) {
                        let _ = content.parse::<kw::event>()?;
                        let _ = content.parse::<Token![=]>()?;
                        event = Some(content.parse::<Type>()?);
                    } else if content.peek(kw::event_converter) {
                        let _ = content.parse::<kw::event_converter>()?;
                        let _ = content.parse::<Token![=]>()?;
                        event_converter = Some(content.parse::<Type>()?);
                    } else if content.peek(kw::callback) {
                        let _ = content.parse::<kw::callback>()?;
                        let _ = content.parse::<Token![=]>()?;
                        callback = Some(content.parse::<Type>()?);
                    } else if content.peek(Token![,]) {
                        let _ = content.parse::<Token![,]>()?;
                    } else {
                        return Err(content.error("Unexpected token"));
                    }
                }

                Ok(EventHandlerArgs::Eigenlayer {
                    instance,
                    event,
                    event_converter,
                    callback,
                })
            }
            _ => Err(syn::Error::new_spanned(
                protocol,
                "Expected `tangle` or `eigenlayer` as event handler protocol",
            )),
        }
    }
}
