use crate::shared;
use crate::shared::{get_return_type_wrapper, pascal_case, type_to_field_type, MacroExt};
use crate::special_impls::evm::{
    generate_evm_specific_impl, get_evm_instance_data, get_evm_job_processor_wrapper,
};
use crate::special_impls::tangle::{
    generate_tangle_specific_impl, get_tangle_job_processor_wrapper,
};
use gadget_blueprint_proc_macro_core::{FieldType, JobDefinition, JobMetadata};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::collections::HashSet;
use std::str::FromStr;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::{Ident, Index, ItemFn, LitInt, Token, Type};

/// Defines custom keywords for defining Job arguments
mod kw {
    syn::custom_keyword!(id);
    syn::custom_keyword!(params);
    syn::custom_keyword!(result);
    syn::custom_keyword!(evm);
    syn::custom_keyword!(event_listener);
    syn::custom_keyword!(listener);
    syn::custom_keyword!(pre_processor);
    syn::custom_keyword!(post_processor);
    syn::custom_keyword!(protocol);
    syn::custom_keyword!(instance);
    syn::custom_keyword!(event);
    syn::custom_keyword!(event_converter);
    syn::custom_keyword!(callback);
    syn::custom_keyword!(abi);
    syn::custom_keyword!(skip_codegen);
}

pub fn get_job_id_field_name(input: &ItemFn) -> (String, Ident, Ident) {
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let job_def_name = format_ident!("{}_JOB_DEF", fn_name_string.to_ascii_uppercase());
    let job_id_name = format_ident!("{}_JOB_ID", fn_name_string.to_ascii_uppercase());
    (fn_name_string, job_def_name, job_id_name)
}

pub fn get_current_call_id_field_name(input: &ItemFn) -> Ident {
    let fn_name = &input.sig.ident;
    format_ident!(
        "{}_ACTIVE_CALL_ID",
        fn_name.to_string().to_ascii_uppercase()
    )
}

/// Job Macro implementation
pub(crate) fn job_impl(args: &JobArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    // Extract function name and arguments
    const SUFFIX: &str = "EventHandler";

    let result = &get_return_type(input);
    let param_map = shared::param_types(&input.sig)?;
    // Extracts Job ID and param/result types
    let job_id = &args.id;
    let params_type = declared_params_to_field_types(&args.params, &param_map)?;
    let result_type = args.result_to_field_types(result)?;

    let (event_listener_gen, event_listener_calls) = generate_event_workflow_tokenstream(
        input,
        SUFFIX,
        &args.event_listener,
        args.skip_codegen,
        &param_map,
        &args.params,
    )?;

    // Generate Event Workflow, if not being skipped
    let additional_specific_logic = if args.skip_codegen {
        proc_macro2::TokenStream::default()
    } else {
        // Specialized code for the event workflow or otherwise
        generate_specialized_logic(
            input,
            &args.event_listener,
            SUFFIX,
            &param_map,
            &args.params,
        )?
    };

    let autogen_struct = if args.skip_codegen {
        proc_macro2::TokenStream::default()
    } else {
        generate_autogen_struct(
            input,
            &args.event_listener,
            &args.params,
            &param_map,
            SUFFIX,
            &event_listener_calls,
        )?
    };

    let job_const_block = generate_job_const_block(input, params_type, result_type, job_id)?;

    // Generates final TokenStream that will be returned
    let gen = quote! {
        #job_const_block

        #autogen_struct

        #(#event_listener_gen)*

        #[allow(unused_variables)]
        #input

        #additional_specific_logic
    };

    // println!("{}", gen.to_string());

    Ok(gen.into())
}

pub fn get_return_type(input: &ItemFn) -> Type {
    match input.sig.output.clone() {
        syn::ReturnType::Type(_, result) => *result,
        syn::ReturnType::Default => {
            let empty_type: Type = syn::parse_quote! { () };
            empty_type
        }
    }
}

pub(crate) trait IsResultType {
    fn is_result_type(&self) -> bool;
}

impl IsResultType for Type {
    fn is_result_type(&self) -> bool {
        if let Type::Path(path) = self {
            if let Some(seg) = path.path.segments.last() {
                seg.ident == "Result"
            } else {
                false
            }
        } else {
            false
        }
    }
}

/// Creates Job Definition using input parameters
pub fn generate_job_const_block(
    input: &ItemFn,
    params: Vec<ParameterType>,
    result: Vec<ParameterType>,
    job_id: &LitInt,
) -> syn::Result<proc_macro2::TokenStream> {
    let (fn_name_string, job_def_name, job_id_name) = get_job_id_field_name(input);
    // Creates Job Definition using input parameters
    let job_def = JobDefinition {
        metadata: JobMetadata {
            name: fn_name_string.clone().into(),
            // filled later on during the rustdoc gen.
            description: None,
        },
        params: params.iter().map(ParameterType::field_type).collect(),
        result: result.iter().map(ParameterType::field_type).collect(),
    };

    // Serialize Job Definition to JSON string
    let job_def_str = serde_json::to_string(&job_def).map_err(|err| {
        syn::Error::new_spanned(
            input,
            format!("Failed to serialize job definition to json: {err}"),
        )
    })?;

    let call_id_static_name = get_current_call_id_field_name(input);
    Ok(quote! {
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

            static #call_id_static_name: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_event_workflow_tokenstream(
    input: &ItemFn,
    suffix: &str,
    event_listeners: &EventListenerArgs,
    skip_codegen: bool,
    param_types: &IndexMap<Ident, Type>,
    params: &[Ident],
) -> syn::Result<(Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>)> {
    let return_type = get_return_type(input);

    let (mut event_handler_args, _event_handler_arg_types) =
        get_event_handler_args(param_types, params)?;
    let (_, _, struct_name) = generate_fn_name_and_struct(input, suffix);
    let (fn_name_string, _job_def_name, job_id_name) = get_job_id_field_name(input);
    // Generate Event Listener, if not being skipped
    let mut event_listener_calls = vec![];
    let event_listener_gen = if skip_codegen {
        vec![proc_macro2::TokenStream::default()]
    } else {
        let mut all_listeners = vec![];
        for (idx, listener_meta) in event_listeners.listeners.iter().enumerate() {
            let listener_function_name = format_ident!(
                "run_listener_{}_{}{}",
                fn_name_string,
                idx,
                suffix.to_lowercase()
            );

            // convert the listener var, which is just a struct name, to an ident
            let listener = listener_meta.listener.to_token_stream();

            // Raw events have no pre-processor, therefore their inputs are passed directly into the job function
            // and NOT as job params
            let is_raw = listener_meta.is_raw();

            // Generate the variable that we are passing as the context into EventListener::create(&mut ctx)
            // We assume the first supplied event handler arg is the context we are injecting into the event listener
            // Then, pass that into the EventFlowWrapper
            let fn_name_ident = &input.sig.ident;
            let static_ctx_get_override = quote! { CTX.get().unwrap() };
            let (ctx_pos_in_ordered_inputs, mut ordered_inputs) =
                get_fn_call_ordered(param_types, params, Some(static_ctx_get_override), is_raw)?;

            let asyncness = get_asyncness(input);
            let call_id_static_name = get_current_call_id_field_name(input);

            // TODO: task 001: find better way to identify which ident is the raw event
            // for now, we assume the raw event is always listed first
            if is_raw {
                let _ = event_handler_args.remove(0);
            }

            let field_in_self_getter = event_handler_args
                .first()
                .map(|field_in_self| {
                    // If is_raw, assume the actual context is the second param
                    quote! { ctx. #field_in_self .clone() }
                })
                .ok_or_else(|| {
                    syn::Error::new(
                        Span::call_site(),
                        "Must specify a context (field_in_self_getter)",
                    )
                })?;

            let autogen_struct_name = quote! { #struct_name };

            /*
            let context_ty = event_handler_arg_types
                .first()
                .map(|ty| quote! {#ty})
                .unwrap_or_default();*/

            if event_listener_calls.is_empty() {
                event_listener_calls.push(quote! {
                    let mut listeners = vec![];
                });
            }

            event_listener_calls.push(quote! {
                listeners.push(#listener_function_name(&self).await.expect("Event listener already initialized"));
            });

            let pre_processor_function = if let Some(preprocessor) = &listener_meta.pre_processor {
                quote! { #preprocessor }
            } else {
                // identity transformation
                get_preprocessor_default_identity_function(&listener_meta.listener_type)
            };

            // The job_processor is just the job function. Since it may contain multiple params, we need a new function to call it.
            let job_processor_wrapper = match listener_meta.listener_type {
                ListenerType::Tangle => get_tangle_job_processor_wrapper(
                    params,
                    param_types,
                    event_listeners,
                    &mut ordered_inputs,
                    fn_name_ident,
                    &asyncness,
                    &return_type,
                    ctx_pos_in_ordered_inputs,
                )?,

                ListenerType::Evm => get_evm_job_processor_wrapper(
                    params,
                    param_types,
                    event_listeners,
                    &mut ordered_inputs,
                    fn_name_ident,
                    &asyncness,
                    &return_type,
                )?,

                ListenerType::Custom => {
                    let job_processor_call_return = get_return_type_wrapper(&return_type);

                    quote! {
                        move |param0| async move {
                            let res = #fn_name_ident (#(#ordered_inputs),*) #asyncness;
                            #job_processor_call_return
                        }
                    }
                }
            };

            let post_processor_function = if let Some(postprocessor) = &listener_meta.post_processor
            {
                match listener_meta.listener_type {
                    ListenerType::Tangle => {
                        quote! {
                            |job_result| async move {
                                let ctx = CTX.get().unwrap();
                                let tangle_job_result = gadget_sdk::event_listener::tangle::TangleResult::<_> {
                                    results: job_result,
                                    service_id: ctx.service_id,
                                    call_id: #call_id_static_name.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                                    client: ctx.client.clone(),
                                    signer: ctx.signer.clone(),
                                };

                                #postprocessor(tangle_job_result).await.map_err(|err| gadget_sdk::Error::Other(err.to_string()))
                            }
                        }
                    }

                    ListenerType::Evm => {
                        quote! { #postprocessor }
                    }

                    ListenerType::Custom => {
                        quote! { #postprocessor }
                    }
                }
            } else {
                // no-op default
                quote! { |_evt| async move { Ok(()) } }
            };

            let context_declaration = match listener_meta.listener_type {
                ListenerType::Tangle => {
                    quote! {
                        let context = gadget_sdk::event_listener::tangle::TangleListenerInput {
                            client: ctx.client.clone(),
                            signer: ctx.signer.clone(),
                            job_id: #job_id_name,
                            service_id: ctx.service_id,
                            context: #field_in_self_getter,
                        };
                    }
                }

                ListenerType::Evm => {
                    quote! { let context = ctx.deref().clone(); }
                }

                ListenerType::Custom => {
                    quote! { let context = #field_in_self_getter; }
                }
            };

            let next_listener = quote! {
                async fn #listener_function_name (ctx: &#autogen_struct_name) -> Option<gadget_sdk::tokio::sync::oneshot::Receiver<Result<(), gadget_sdk::Error>>> {
                    static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                    if !ONCE.fetch_or(true, std::sync::atomic::Ordering::Relaxed) {
                        let (tx, rx) = gadget_sdk::tokio::sync::oneshot::channel();

                        static CTX: gadget_sdk::tokio::sync::OnceCell<#autogen_struct_name> = gadget_sdk::tokio::sync::OnceCell::const_new();
                        #context_declaration

                        if let Err(_err) = CTX.set(ctx.clone()) {
                            gadget_sdk::error!("Failed to set the context");
                            return None;
                        }
                        let job_processor = #job_processor_wrapper;

                        let listener = <#listener as gadget_sdk::event_listener::EventListener<_, _>>::new(&context).await.expect("Failed to create event listener");
                        let mut event_workflow = gadget_sdk::event_listener::executor::EventFlowWrapper::new(
                            listener,
                            #pre_processor_function,
                            job_processor,
                            #post_processor_function,
                        );

                        let task = async move {
                            let res = gadget_sdk::event_listener::executor::EventFlowExecutor::event_loop(&mut event_workflow).await;
                            let _ = tx.send(res);
                        };
                        gadget_sdk::tokio::task::spawn(task);
                        return Some(rx)
                    }

                    None
                }
            };

            all_listeners.push(next_listener);
        }

        all_listeners
    };

    if event_listener_calls.is_empty() {
        event_listener_calls.push(proc_macro2::TokenStream::default())
    }

    Ok((event_listener_gen, event_listener_calls))
}

fn get_preprocessor_default_identity_function(
    listener_type: &ListenerType,
) -> proc_macro2::TokenStream {
    match listener_type {
        ListenerType::Evm => {
            quote! {
                |evt| async move {
                    Ok(evt)
                }
            }
        }
        ListenerType::Tangle => {
            quote! {
                |evt| async move {
                    Ok(evt)
                }
            }
        }
        ListenerType::Custom => {
            // Identity transform
            quote! {
                |evt| async move {
                    Ok(evt)
                }
            }
        }
    }
}

/// Get all the params names inside the param_types map
/// and not in the params list to be added to the event handler.
pub(crate) fn get_event_handler_args<'a>(
    param_types: &'a IndexMap<Ident, Type>,
    params: &'a [Ident],
) -> syn::Result<(Vec<&'a Ident>, Vec<&'a Type>)> {
    let x = param_types.keys().collect::<IndexSet<_>>();
    let y = params.iter().collect::<IndexSet<_>>();
    let event_handler_args = x.difference(&y).copied().collect::<Vec<_>>();
    let event_handler_types = Itertools::try_collect(event_handler_args.iter().map(|r| {
        param_types.get(*r).ok_or_else(|| {
            syn::Error::new_spanned(
                r,
                "Could not find the type of the parameter in the function signature",
            )
        })
    }))?;

    Ok((event_handler_args, event_handler_types))
}

fn generate_fn_name_and_struct<'a>(f: &'a ItemFn, suffix: &'a str) -> (&'a Ident, String, Ident) {
    let fn_name = &f.sig.ident;
    let fn_name_string = fn_name.to_string();
    let struct_name = format_ident!("{}{}", pascal_case(&fn_name_string), suffix);
    (fn_name, fn_name_string, struct_name)
}

#[allow(clippy::too_many_lines)]
pub fn generate_autogen_struct(
    input: &ItemFn,
    event_listener_args: &EventListenerArgs,
    params: &[Ident],
    param_types: &IndexMap<Ident, Type>,
    suffix: &str,
    event_listener_calls: &[proc_macro2::TokenStream],
) -> syn::Result<proc_macro2::TokenStream> {
    let (_fn_name, fn_name_string, struct_name) = generate_fn_name_and_struct(input, suffix);

    let (event_handler_args, _) = get_event_handler_args(param_types, params)?;

    let mut additional_var_indexes = vec![];
    let mut additional_params = event_handler_args
        .iter()
        .map(|ident| {
            let mut ty = param_types[*ident].clone();
            additional_var_indexes.push(param_types.get_index_of(*ident).expect("Should exist"));
            // remove the reference from the type and use the inner type
            if let Type::Reference(r) = ty {
                ty = *r.elem;
            }

            quote! {
                pub #ident: #ty,
            }
        })
        .collect::<Vec<_>>();

    // TODO: task 001: find better way to identify which ident is the raw event
    // for now, we assume the raw event is always listed first
    if event_listener_args.get_event_listener().is_raw() {
        // We don't care to add the first event to the autogen struct
        let _ = additional_var_indexes.remove(0);
        let _ = additional_params.remove(0);
    }

    let mut required_fields = vec![];

    // Even if multiple tangle listeners, we only need this once
    if event_listener_args.has_tangle() {
        required_fields.push(quote! {
            pub service_id: u64,
            pub signer: gadget_sdk::keystore::TanglePairSigner<gadget_sdk::ext::sp_core::sr25519::Pair>,
            pub client: gadget_sdk::clients::tangle::runtime::TangleClient,
        })
    }

    // Even if multiple evm listeners, we only need this once
    if event_listener_args.has_evm() {
        let (_, _, _, instance_name) = get_evm_instance_data(event_listener_args)?;

        required_fields.push(quote! {
            pub contract: #instance_name,
            pub contract_instance: std::sync::OnceLock<gadget_sdk::event_listener::evm::contracts::AlloyContractInstance>,
        });
    }

    let combined_event_listener = generate_combined_event_listener_selector(&struct_name);

    Ok(quote! {
        /// Event handler for the function
        #[doc = "[`"]
        #[doc = #fn_name_string]
        #[doc = "`]"]
        #[derive(Clone)]
        pub struct #struct_name {
            #(#required_fields)*
            #(#additional_params)*
        }

        #[gadget_sdk::async_trait::async_trait]
        impl gadget_sdk::event_utils::InitializableEventHandler for #struct_name {
            async fn init_event_handler(
                &self,
            ) -> Option<gadget_sdk::tokio::sync::oneshot::Receiver<Result<(), gadget_sdk::Error>>> {
                #(#event_listener_calls)*
                #combined_event_listener
            }
        }
    })
}

pub fn get_fn_call_ordered(
    param_types: &IndexMap<Ident, Type>,
    params_from_job_args: &[Ident],
    replacement_for_self: Option<proc_macro2::TokenStream>,
    is_raw: bool,
) -> syn::Result<(usize, Vec<proc_macro2::TokenStream>)> {
    let (event_handler_args, _) = get_event_handler_args(param_types, params_from_job_args)?;

    let additional_var_indexes: Vec<usize> =
        Itertools::try_collect(event_handler_args.iter().map(|ident| {
            param_types.get_index_of(*ident).ok_or_else(|| {
                syn::Error::new_spanned(
                    ident,
                    "Could not find the index of the parameter in the function signature",
                )
            })
        }))?;

    // This has all params
    let mut job_var_idx = 0;
    let mut non_job_var_count = 0;
    let mut ctx_pos = None;
    let this = replacement_for_self.unwrap_or_else(|| quote! { self });
    let ret = param_types
        .iter()
        .enumerate()
        .map(|(pos_in_all_args, (ident, ty))| {
            // if the current param is not in the additional params, then it is a job param to be passed to the function

            let is_job_var = !additional_var_indexes.contains(&pos_in_all_args);

            if is_job_var {
                let ident = format_ident!("param{job_var_idx}");
                job_var_idx += 1;
                return quote! { #ident };
            }

            let (is_ref, is_ref_mut) = match ty {
                Type::Reference(r) => (true, r.mutability.is_some()),
                _ => (false, false),
            };

            if non_job_var_count == 0 {
                // Assume first non-job var is the context
                ctx_pos = Some(pos_in_all_args);
            }

            non_job_var_count += 1;

            if is_ref && is_ref_mut {
                quote! { &mut #this .#ident }
            } else if is_ref {
                quote! { &#this .#ident }
            } else {
                quote! { #this .#ident.clone() }
            }
        })
        .collect::<Vec<_>>();

    if is_raw {
        ctx_pos = Some(1); // Raw events have only two events: 0 = the raw event, 1 = the context
    }

    let ctx_pos = ctx_pos.ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            "Could not find the context in the function signature",
        )
    })?;

    Ok((ctx_pos, ret))
}

fn get_asyncness(input: &ItemFn) -> proc_macro2::TokenStream {
    if input.sig.asyncness.is_some() {
        quote! {.await}
    } else {
        quote! {}
    }
}

/// Generates the [`EventHandler`](gadget_sdk::event_utils::evm::EventHandler) for a Job
#[allow(clippy::too_many_lines)]
pub fn generate_specialized_logic(
    input: &ItemFn,
    event_listener_args: &EventListenerArgs,
    suffix: &str,
    param_map: &IndexMap<Ident, Type>,
    job_params: &[Ident],
) -> syn::Result<proc_macro2::TokenStream> {
    let (_fn_name, _fn_name_string, struct_name) = generate_fn_name_and_struct(input, suffix);

    match event_listener_args.get_event_listener().listener_type {
        ListenerType::Evm => {
            generate_evm_specific_impl(&struct_name, event_listener_args, param_map, job_params)
        }

        ListenerType::Tangle => {
            generate_tangle_specific_impl(&struct_name, param_map, job_params, event_listener_args)
        }

        ListenerType::Custom => Ok(proc_macro2::TokenStream::default()),
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
    /// List of return types for the job, could be inferred from the function return type.
    /// `#[job(result(u32, u64))]`
    /// `#[job(result(_))]`
    result: ResultsKind,
    /// Optional: Event listener type for the job
    /// `#[job(event_listener(MyCustomListener))]`
    event_listener: EventListenerArgs,
    /// Optional: Skip code generation for this job.
    /// `#[job(skip_codegen)]`
    /// this is useful if the developer want to impl a custom event handler
    /// for this job.
    skip_codegen: bool,
}

impl MacroExt for JobArgs {
    fn return_type(&self) -> &ResultsKind {
        &self.result
    }
}

impl Parse for JobArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut params = Vec::new();
        let mut result = None;
        let mut id = None;
        let mut skip_codegen = false;
        let mut event_listener = EventListenerArgs { listeners: vec![] };

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
            } else if lookahead.peek(kw::skip_codegen) {
                let _ = input.parse::<kw::skip_codegen>()?;
                skip_codegen = true;
            } else if lookahead.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            } else if lookahead.peek(kw::event_listener) {
                let next_event_listener: EventListenerArgs = input.parse()?;
                event_listener
                    .listeners
                    .extend(next_event_listener.listeners);
            } else {
                return Err(lookahead.error());
            }
        }

        let id = id.ok_or_else(|| input.error("Missing `id` argument in attribute"))?;

        let result = result.unwrap_or(ResultsKind::Infered);

        if let ResultsKind::Types(ref r) = result {
            if r.is_empty() {
                return Err(input.error("`result` attribute empty, expected at least one parameter, or `_` to infer the type, or to leave omit this field entirely"));
            }
        }

        if event_listener.listeners.is_empty() {
            return Err(input.error("Missing `event_listener` argument in attribute"));
        }

        if event_listener.listeners.len() > 1 {
            return Err(input.error("Only one event listener is currently allowed"));
        }

        Ok(JobArgs {
            id,
            params,
            result,
            skip_codegen,
            event_listener,
        })
    }
}

/// Contains the Job parameters as a [`Vector`](Vec) of Identifers ([`Ident`](proc_macro2::Ident))
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

#[derive(Debug, Clone)]
pub struct ParameterType {
    pub ty: FieldType,
    pub span: Option<Span>,
}

impl PartialEq for ParameterType {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty
    }
}

impl Eq for ParameterType {}

impl ParameterType {
    pub(crate) fn field_type(&self) -> FieldType {
        self.ty.clone()
    }
}

pub(crate) fn declared_params_to_field_types(
    params: &[Ident],
    param_types: &IndexMap<Ident, Type>,
) -> syn::Result<Vec<ParameterType>> {
    let mut ret = Vec::new();
    for param in params {
        let ty = param_types.get(param).ok_or_else(|| {
            syn::Error::new_spanned(param, "parameter not declared in the function")
        })?;

        ret.push(type_to_field_type(ty)?)
    }
    Ok(ret)
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
pub(crate) struct Results(pub(crate) ResultsKind);

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

/// `#[job(event_listener(MyCustomListener, MyCustomListener2)]`
/// Accepts an optional argument that specifies the event listener to use that implements EventListener
pub(crate) struct EventListenerArgs {
    pub(crate) listeners: Vec<SingleListener>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ListenerType {
    Evm,
    Tangle,
    Custom,
}

pub(crate) struct SingleListener {
    pub listener: Type,
    pub evm_args: Option<EvmArgs>,
    pub listener_type: ListenerType,
    pub post_processor: Option<Type>,
    pub pre_processor: Option<Type>,
}

impl SingleListener {
    pub fn is_raw(&self) -> bool {
        self.pre_processor.is_none() && matches!(self.listener_type, ListenerType::Tangle)
    }
}

/// Extracts a value from form: "tag = value"
fn extract_x_equals_y<T: Parse, P: Parse>(
    content: &ParseBuffer,
    required: bool,
    name: &str,
) -> syn::Result<Option<P>> {
    if content.peek(Token![,]) {
        let _ = content.parse::<Token![,]>()?;
    }

    if content.parse::<T>().is_err() {
        if required {
            panic!("Expected keyword {name}, none supplied")
        } else {
            return Ok(None);
        }
    }

    if !content.peek(Token![=]) {
        panic!("Expected = after variable {name}")
    }

    let _ = content.parse::<Token![=]>()?;

    let listener = content.parse::<P>()?;
    Ok(Some(listener))
}

const EVM_EVENT_LISTENER_TAG: &str = "EvmContractEventListener";
const TANGLE_EVENT_LISTENER_TAG: &str = "TangleEventListener";

impl Parse for EventListenerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<kw::event_listener>()?;
        let content;
        syn::parenthesized!(content in input);

        let mut listener = None;
        let mut pre_processor = None;
        let mut post_processor = None;
        let mut is_evm = false;
        // EVM specific
        let mut instance: Option<Ident> = None;
        let mut abi: Option<Type> = None;

        while !content.is_empty() {
            if content.peek(kw::listener) {
                let listener_found =
                    extract_x_equals_y::<kw::listener, Type>(&content, true, "listener")?
                        .ok_or_else(|| content.error("Expected `listener` field"))?;

                let ty_str = quote! { #listener_found }.to_string();

                if ty_str.contains(EVM_EVENT_LISTENER_TAG) {
                    is_evm = true;
                }

                listener = Some(listener_found)
            } else if content.peek(kw::pre_processor) {
                pre_processor = extract_x_equals_y::<kw::pre_processor, Type>(
                    &content,
                    false,
                    "pre_processor",
                )?;
            } else if content.peek(kw::post_processor) {
                post_processor = extract_x_equals_y::<kw::post_processor, Type>(
                    &content,
                    false,
                    "post_processor",
                )?;
            } else if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>()?;
            } else if content.peek(kw::instance) {
                let _ = content.parse::<kw::instance>()?;
                let _ = content.parse::<Token![=]>()?;
                instance = Some(content.parse::<Ident>()?);
            } else if content.peek(kw::abi) {
                let _ = content.parse::<kw::abi>()?;
                let _ = content.parse::<Token![=]>()?;
                abi = Some(content.parse::<Type>()?);
            } else {
                return Err(content.error(
                    "Unexpected field parsed. Expected one of `listener`, `event`, `pre_processor`, `post_processor`",
                ));
            }
        }

        let listener = listener.ok_or_else(|| content.error("Expected `listener` argument"))?;
        // Create a listener. If this is an EvmContractEventListener, we need to specially parse the arguments
        // In the case of tangle and everything other listener type, we don't pass evm_args
        let ty_str = quote! { #listener }.to_string();
        let this_listener = if is_evm {
            if instance.is_none() {
                return Err(content.error("Expected `instance` argument for EVM event listener"));
            }

            if abi.is_none() {
                return Err(content.error("Expected `abi` argument for EVM event listener"));
            }

            SingleListener {
                listener,
                evm_args: Some(EvmArgs { instance, abi }),
                listener_type: ListenerType::Evm,
                post_processor,
                pre_processor,
            }
        } else {
            let listener_type = if ty_str.contains(TANGLE_EVENT_LISTENER_TAG) {
                ListenerType::Tangle
            } else {
                ListenerType::Custom
            };

            SingleListener {
                listener,
                evm_args: None,
                listener_type,
                post_processor,
                pre_processor,
            }
        };

        Ok(Self {
            listeners: vec![this_listener],
        })
    }
}

pub(crate) struct EvmArgs {
    pub instance: Option<Ident>,
    pub abi: Option<Type>,
}

impl EventListenerArgs {
    pub fn get_event_listener(&self) -> &SingleListener {
        &self.listeners[0]
    }

    pub fn get_param_name_tokenstream(
        &self,
        params: &[ParameterType],
    ) -> Vec<proc_macro2::TokenStream> {
        let listener_type = self.get_event_listener().listener_type;

        params
            .iter()
            .enumerate()
            .map(|(i, param_ty)| {
                let ident = format_ident!("param{i}");
                let index = Index::from(i);
                match listener_type {
                    ListenerType::Tangle => {
                        let ty_token_stream = proc_macro2::TokenStream::from_str(&param_ty.ty.as_rust_type()).expect("should be valid");
                        let ty_tokens = quote_spanned! {param_ty.span.expect("should always be available")=>
                            #ty_token_stream
                        };
                        quote! {
                            let __arg = args.next().expect("parameter count checked before");
                            let Ok(#ident) = ::gadget_sdk::ext::blueprint_serde::from_field::<#ty_tokens>(__arg) else {
                                return Err(::gadget_sdk::Error::BadArgumentDecoding(format!("Failed to decode the field `{}` to `{}`", stringify!(#ident), stringify!(#ty_tokens))));
                            };
                        }
                    }
                    ListenerType::Evm => {
                        quote! {
                            let #ident = inputs.#index;
                        }
                    }
                    // All other event listeners will return just one type
                    ListenerType::Custom => {
                        quote! {
                            let #ident = inputs.#index;
                        }
                    }
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn has_tangle(&self) -> bool {
        self.listeners
            .iter()
            .any(|r| r.listener_type == ListenerType::Tangle)
    }

    pub fn has_evm(&self) -> bool {
        self.listeners
            .iter()
            .any(|r| r.listener_type == ListenerType::Evm)
    }

    /// Returns the Event Handler's Contract Instance on the EVM.
    pub fn instance(&self) -> Option<Ident> {
        match self.get_event_listener().evm_args.as_ref() {
            Some(EvmArgs { instance, .. }) => instance.clone(),
            None => None,
        }
    }
}

impl Parse for EvmArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);

        let mut instance = None;
        let mut abi = None;

        while !content.is_empty() {
            if content.peek(kw::instance) {
                let _ = content.parse::<kw::instance>()?;
                let _ = content.parse::<Token![=]>()?;
                instance = Some(content.parse::<Ident>()?);
            } else if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>()?;
            } else if content.peek(kw::abi) {
                let _ = content.parse::<kw::abi>()?;
                let _ = content.parse::<Token![=]>()?;
                abi = Some(content.parse::<Type>()?);
            } else {
                return Err(content.error("Unexpected token"));
            }
        }

        Ok(EvmArgs { instance, abi })
    }
}

pub(crate) fn generate_combined_event_listener_selector(
    struct_name: &Ident,
) -> proc_macro2::TokenStream {
    quote! {
        let (tx, rx) = gadget_sdk::tokio::sync::oneshot::channel();
        let task = async move {
            let mut futures = gadget_sdk::futures::stream::FuturesUnordered::new();
            for listener in listeners {
                futures.push(listener);
            }
            if let Some(res) = gadget_sdk::futures::stream::StreamExt::next(&mut futures).await {
                gadget_sdk::warn!("An Event Handler for {} has stopped running", stringify!(#struct_name));
                let res = match res {
                    Ok(res) => {
                        res
                    },
                    Err(e) => {
                        Err(gadget_sdk::Error::Other(format!("Error in Event Handler for {}: {e:?}", stringify!(#struct_name))))
                    }
                };

                tx.send(res).unwrap();
            }
        };
        let _ = gadget_sdk::tokio::spawn(task);
        Some(rx)
    }
}
