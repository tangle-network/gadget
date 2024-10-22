use crate::event_listener::evm::{generate_evm_event_handler, get_evm_instance_data};
use crate::event_listener::tangle::generate_tangle_event_handler;
use crate::shared::{pascal_case, type_to_field_type};
use gadget_blueprint_proc_macro_core::{FieldType, JobDefinition, JobMetadata};
use indexmap::{IndexMap, IndexSet};
use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use std::collections::HashSet;
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
    syn::custom_keyword!(skip_codegen);
}

pub fn get_job_id_field_name(input: &ItemFn) -> (String, Ident, Ident) {
    let fn_name = &input.sig.ident;
    let fn_name_string = fn_name.to_string();
    let job_def_name = format_ident!("{}_JOB_DEF", fn_name_string.to_ascii_uppercase());
    let job_id_name = format_ident!("{}_JOB_ID", fn_name_string.to_ascii_uppercase());
    (fn_name_string, job_def_name, job_id_name)
}

/// Job Macro implementation
pub(crate) fn job_impl(args: &JobArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    // Extract function name and arguments
    let (fn_name_string, job_def_name, job_id_name) = get_job_id_field_name(input);
    const SUFFIX: &str = "EventHandler";

    let syn::ReturnType::Type(_, result) = &input.sig.output else {
        return Err(syn::Error::new_spanned(
            &input.sig.output,
            "Function must have a return type of Result<T, E> where T is a tuple of the result fields",
        ));
    };

    // Check that the function has a return type of Result<T, E>
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

    let param_types = param_types(input)?;
    let (event_listener_gen, event_listener_calls) = generate_event_listener_tokenstream(
        input,
        SUFFIX,
        &fn_name_string,
        &args.event_listener,
        args.skip_codegen,
        &param_types,
        &args.params,
    );

    // Extracts Job ID and param/result types
    let job_id = &args.id;
    let params_type = args.params_to_field_types(&param_types)?;
    let result_type = args.result_to_field_types(result)?;

    // Generate Event Workflow, if not being skipped
    let event_workflow_gen = if args.skip_codegen {
        proc_macro2::TokenStream::default()
    } else {
        // Specialized code for the event workflow
        generate_additional_logic(
            input,
            args,
            &param_types,
            &params_type,
            &result_type,
            SUFFIX,
        )
    };

    let autogen_struct = if args.skip_codegen {
        proc_macro2::TokenStream::default()
    } else {
        generate_autogen_struct(input, args, &param_types, SUFFIX, &event_listener_calls)
    };

    // Creates Job Definition using input parameters
    let job_def = JobDefinition {
        metadata: JobMetadata {
            name: fn_name_string.clone().into(),
            // filled later on during the rustdoc gen.
            description: None,
        },
        params: params_type,
        result: result_type,
    };

    // Serialize Job Definition to JSON string
    let job_def_str = serde_json::to_string(&job_def).map_err(|err| {
        syn::Error::new_spanned(
            input,
            format!("Failed to serialize job definition to json: {err}"),
        )
    })?;

    // Generates final TokenStream that will be returned
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

        #autogen_struct

        #(#event_listener_gen)*

        #[allow(unused_variables)]
        #input

        #event_workflow_gen
    };

    // println!("{}", gen.to_string());

    Ok(gen.into())
}

pub(crate) fn param_types(input: &ItemFn) -> syn::Result<IndexMap<Ident, Type>> {
    // Ensures that no duplicate parameters have been given
    let mut param_types = IndexMap::new();
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

    Ok(param_types)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_event_listener_tokenstream(
    input: &ItemFn,
    suffix: &str,
    fn_name_string: &str,
    event_listeners: &EventListenerArgs,
    skip_codegen: bool,
    param_types: &IndexMap<Ident, Type>,
    params: &[Ident],
) -> (Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>) {
    let (event_handler_args, event_handler_arg_types) = get_event_handler_args(param_types, params);
    let (_, _, struct_name) = generate_fn_name_and_struct(input, suffix);
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
            let is_tangle = listener_meta.evm_args.is_none();
            // convert the listener var, which is just a struct name, to an ident
            let listener = listener_meta.listener.to_token_stream();
            // if Listener == TangleEventListener or EvmContractEventListener, we need to use defaults
            let listener_str = listener.to_string();

            let type_args = if is_tangle {
                proc_macro2::TokenStream::default()
            } else {
                quote! { <T> }
            };

            let bounded_type_args = if is_tangle {
                proc_macro2::TokenStream::default()
            } else {
                quote! { <T: Clone + Send + Sync + gadget_sdk::events_watcher::evm::Config +'static> }
            };

            let autogen_struct_name = quote! { #struct_name #type_args };

            // Check for special cases
            let next_listener = if listener_str.contains("TangleEventListener")
                || listener_str.contains("EvmContractEventListener")
            {
                // How to inject not just this event handler, but all event handlers here?
                let wrapper = if is_tangle {
                    quote! {
                        gadget_sdk::event_listener::TangleEventWrapper<_>
                    }
                } else {
                    quote! {
                        gadget_sdk::event_listener::EthereumHandlerWrapper<#autogen_struct_name, _>
                    }
                };

                let ctx_create = if is_tangle {
                    quote! {
                        (ctx.client.clone(), std::sync::Arc::new(ctx.clone()) as gadget_sdk::events_watcher::substrate::EventHandlerFor<gadget_sdk::clients::tangle::runtime::TangleConfig, _>)
                    }
                } else {
                    quote! {
                        (ctx.contract.clone(), std::sync::Arc::new(ctx.clone()) as std::sync::Arc<#autogen_struct_name>)
                    }
                };

                if event_listener_calls.is_empty() {
                    event_listener_calls.push(quote! {
                        let mut listeners = vec![];
                    });
                }

                event_listener_calls.push(quote! {
                            listeners.push(#listener_function_name(&self).await.expect("Event listener already initialized"));
                        });

                quote! {
                    async fn #listener_function_name #bounded_type_args(ctx: &#autogen_struct_name) -> Option<gadget_sdk::tokio::sync::oneshot::Receiver<Result<(), gadget_sdk::Error>>>{
                        static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                        if !ONCE.load(std::sync::atomic::Ordering::Relaxed) {
                            ONCE.store(true, std::sync::atomic::Ordering::Relaxed);
                            let (tx, rx) = gadget_sdk::tokio::sync::oneshot::channel();
                            let ctx = #ctx_create;
                            let mut instance = <#wrapper as gadget_sdk::event_listener::EventListener::<_, _>>::new(&ctx).await.expect("Failed to create event listener");
                            let task = async move {
                                let res = gadget_sdk::event_listener::EventListener::<_, _>::execute(&mut instance).await;
                                let _ = tx.send(res);
                            };
                            gadget_sdk::tokio::task::spawn(task);
                            return Some(rx)
                        }

                        None
                    }
                }
            } else {
                // Generate the variable that we are passing as the context into EventListener::create(&mut ctx)
                // We assume the first supplied event handler arg is the context we are injecting into the event listener
                let (context, field_in_self) = event_handler_args
                    .first()
                    .map(|ctx| (quote! {self}, (*ctx).clone()))
                    .expect("No context found");

                let context_ty = event_handler_arg_types
                    .first()
                    .map(|_ty| quote! {#struct_name})
                    .unwrap_or_default();

                if event_listener_calls.is_empty() {
                    event_listener_calls.push(quote! {
                        let mut listeners = vec![];
                    });
                }

                event_listener_calls.push(quote! {
                    listeners.push(#listener_function_name(&#context).await.expect("Event listener already initialized"));
                });

                let event_type = &listener_meta.event;
                // The event type is what gets sent through the pre_processor_function.

                let pre_processor_function =
                    if let Some(preprocessor) = &listener_meta.pre_processor {
                        quote! { #preprocessor }
                    } else {
                        // identity transformation
                        quote! { |evt| async move { Ok(evt) } }
                    };

                // The job_processor is just the job function. Since it may contain multiple params, we need a new function to call it.
                let fn_name_ident = &input.sig.ident;
                let ordered_inputs =
                    get_fn_call_ordered(param_types, params, Some(quote! { CTX.get().unwrap() }));
                let asyncness = get_asyncness(input);
                // The below assumes param0 IS the event streamed from the event listener
                let job_processor_wrapper = quote! {
                    move |param0| async move {
                        #fn_name_ident (#(#ordered_inputs)*) #asyncness .map_err(|err| gadget_sdk::Error::Other(err.to_string()))
                    }
                };
                let post_processor_function =
                    if let Some(postprocessor) = &listener_meta.post_processor {
                        quote! { #postprocessor }
                    } else {
                        // no-op default
                        quote! { |_evt| async move { Ok(()) } }
                    };

                quote! {
                    async fn #listener_function_name(ctx: &#context_ty) -> Option<gadget_sdk::tokio::sync::oneshot::Receiver<Result<(), gadget_sdk::Error>>> {
                        static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                        if !ONCE.load(std::sync::atomic::Ordering::Relaxed) {
                            ONCE.store(true, std::sync::atomic::Ordering::Relaxed);
                            let (tx, rx) = gadget_sdk::tokio::sync::oneshot::channel();
                            static CTX: gadget_sdk::tokio::sync::OnceCell<#context_ty> = gadget_sdk::tokio::sync::OnceCell::const_new();
                            if let Err(_err) = CTX.set(ctx.clone()) {
                                gadget_sdk::error!("Failed to set the context");
                                return None;
                            }
                            let job_processor = #job_processor_wrapper;

                            let listener = <#listener as gadget_sdk::event_listener::EventListener<#event_type, _>>::new(&ctx. #field_in_self).await.expect("Failed to create event listener");
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
                }
            };

            all_listeners.push(next_listener);
        }

        all_listeners
    };

    if event_listener_calls.is_empty() {
        event_listener_calls.push(proc_macro2::TokenStream::default())
    }

    (event_listener_gen, event_listener_calls)
}

/// Get all the params names inside the param_types map
/// and not in the params list to be added to the event handler.
pub(crate) fn get_event_handler_args<'a>(
    param_types: &'a IndexMap<Ident, Type>,
    params: &'a [Ident],
) -> (Vec<&'a Ident>, Vec<&'a Type>) {
    let x = param_types.keys().collect::<IndexSet<_>>();
    let y = params.iter().collect::<IndexSet<_>>();
    let event_handler_args = x.difference(&y).copied().collect::<Vec<_>>();
    let event_handler_types = event_handler_args
        .iter()
        .map(|r| param_types.get(*r).unwrap())
        .collect::<Vec<_>>();
    (event_handler_args, event_handler_types)
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
    job_args: &JobArgs,
    param_types: &IndexMap<Ident, Type>,
    suffix: &str,
    event_listener_calls: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let (_fn_name, fn_name_string, struct_name) = generate_fn_name_and_struct(input, suffix);

    let (event_handler_args, _) = get_event_handler_args(param_types, &job_args.params);

    let mut additional_var_indexes = vec![];
    let additional_params = event_handler_args
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

    let mut required_fields = vec![];
    let mut type_params_bounds = proc_macro2::TokenStream::default();
    let mut type_params = proc_macro2::TokenStream::default();

    // Even if multiple tangle listeners, we only need this once
    if job_args.event_listener.has_tangle() {
        required_fields.push(quote! {
            pub service_id: u64,
            pub signer: gadget_sdk::keystore::TanglePairSigner<gadget_sdk::ext::sp_core::sr25519::Pair>,
            pub client: gadget_sdk::clients::tangle::runtime::TangleClient,
        })
    }

    // Even if multiple evm listeners, we only need this once
    if job_args.event_listener.has_evm() {
        let (_, _, instance_wrapper_name, _) = get_evm_instance_data(&job_args.event_listener);

        required_fields.push(quote! {
            pub contract: #instance_wrapper_name<T::TH, T::PH>,
        });

        type_params = quote! { <T> };
        type_params_bounds =
            quote! { <T: Clone + Send + Sync + gadget_sdk::events_watcher::evm::Config + 'static> };
    }

    let combined_event_listener = generate_combined_event_listener_selector(&struct_name);

    quote! {
        /// Event handler for the function
        #[doc = "[`"]
        #[doc = #fn_name_string]
        #[doc = "`]"]
        #[derive(Clone)]
        pub struct #struct_name #type_params_bounds {
            #(#required_fields)*
            #(#additional_params)*
        }

        #[async_trait::async_trait]
        impl #type_params_bounds gadget_sdk::events_watcher::InitializableEventHandler for #struct_name #type_params {
            async fn init_event_handler(
                &self,
            ) -> Option<gadget_sdk::tokio::sync::oneshot::Receiver<Result<(), gadget_sdk::Error>>> {
                #(#event_listener_calls)*
                #combined_event_listener
            }
        }
    }
}

pub fn get_fn_call_ordered(
    param_types: &IndexMap<Ident, Type>,
    params_from_job_args: &[Ident],
    replacement_for_self: Option<proc_macro2::TokenStream>,
) -> Vec<proc_macro2::TokenStream> {
    let (event_handler_args, _) = get_event_handler_args(param_types, params_from_job_args);

    let additional_var_indexes = event_handler_args
        .iter()
        .map(|ident| param_types.get_index_of(*ident).expect("Should exist"))
        .collect::<Vec<_>>();

    // This has all params
    let mut job_var_idx = 0;
    let this = replacement_for_self.unwrap_or_else(|| quote! { self });
    param_types
        .iter()
        .enumerate()
        .map(|(pos_in_all_args, (ident, ty))| {
            // if the current param is not in the additional params, then it is a job param to be passed to the function

            let is_job_var = !additional_var_indexes.contains(&pos_in_all_args);

            if is_job_var {
                let ident = format_ident!("param{job_var_idx}");
                job_var_idx += 1;
                return quote! { #ident, };
            }

            let (is_ref, is_ref_mut) = match ty {
                Type::Reference(r) => (true, r.mutability.is_some()),
                _ => (false, false),
            };

            if is_ref && is_ref_mut {
                quote! { &mut #this .#ident, }
            } else if is_ref {
                quote! { &#this .#ident, }
            } else {
                quote! { #this .#ident.clone(), }
            }
        })
        .collect::<Vec<_>>()
}

fn get_asyncness(input: &ItemFn) -> proc_macro2::TokenStream {
    if input.sig.asyncness.is_some() {
        quote! {.await}
    } else {
        quote! {}
    }
}

/// Generates the [`EventHandler`](gadget_sdk::events_watcher::evm::EventHandler) for a Job
#[allow(clippy::too_many_lines)]
pub fn generate_additional_logic(
    input: &ItemFn,
    job_args: &JobArgs,
    param_types: &IndexMap<Ident, Type>,
    params: &[FieldType],
    results: &[FieldType],
    suffix: &str,
) -> proc_macro2::TokenStream {
    let (fn_name, _fn_name_string, struct_name) = generate_fn_name_and_struct(input, suffix);
    let job_id = &job_args.id;
    let event_listener_args = &job_args.event_listener;
    let params_tokens = job_args.event_listener.get_param_name_tokenstream(params);
    let fn_call_ordered = get_fn_call_ordered(param_types, &job_args.params, None);

    let asyncness = get_asyncness(input);

    let fn_call = quote! {
        let job_result = match #fn_name(
            #(#fn_call_ordered)*
        )#asyncness {
            Ok(r) => r,
            Err(e) => {
                ::gadget_sdk::error!("Error in job: {e}");
                let error = gadget_sdk::events_watcher::Error::Handler(Box::new(e));
                return Err(error);
            }
        };
    };

    let result_tokens = job_args
        .event_listener
        .get_param_result_tokenstream(results);

    match job_args.event_listener.get_event_listener().listener_type {
        ListenerType::Evm => {
            generate_evm_event_handler(&struct_name, event_listener_args, &params_tokens, &fn_call)
        }

        ListenerType::Tangle => generate_tangle_event_handler(
            &struct_name,
            job_id,
            &params_tokens,
            &result_tokens,
            &fn_call,
        ),

        ListenerType::Custom => proc_macro2::TokenStream::default(),
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
                event_listener = input.parse()?;
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

impl JobArgs {
    fn params_to_field_types(
        &self,
        param_types: &IndexMap<Ident, Type>,
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
/// `#[job(event_listener(MyCustomListener, MyCustomListener2)]`
/// Accepts an optional argument that specifies the event listener to use that implements EventListener
pub(crate) struct EventListenerArgs {
    listeners: Vec<SingleListener>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ListenerType {
    Evm,
    Tangle,
    Custom,
}

#[derive(Debug)]
pub(crate) struct SingleListener {
    pub listener: Type,
    pub evm_args: Option<EvmArgs>,
    pub listener_type: ListenerType,
    pub event: Type,
    pub post_processor: Option<Ident>,
    pub pre_processor: Option<Ident>,
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

impl Parse for EventListenerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<kw::event_listener>()?;
        let content;
        syn::parenthesized!(content in input);

        let mut listeners = Vec::new();
        // Parse a TypePath instead of a LitStr
        while !content.is_empty() {
            let listener = extract_x_equals_y::<kw::listener, Type>(&content, true, "listener")?
                .expect("No listener defined in listener block");

            let ty_str = quote! { #listener }.to_string();
            let mut evm_args = None;
            if ty_str.contains("EvmContractEventListener") {
                evm_args = Some(content.parse::<EvmArgs>()?);
            }

            let event = extract_x_equals_y::<kw::event, Type>(&content, true, "event")?
                .expect("No event defined in listener block");
            let pre_processor =
                extract_x_equals_y::<kw::pre_processor, Ident>(&content, false, "pre_processor")?;
            let post_processor =
                extract_x_equals_y::<kw::post_processor, Ident>(&content, false, "post_processor")?;

            // Create a listener. If this is an EvmContractEventListener, we need to specially parse the arguments
            // In the case of tangle and everything other listener type, we don't pass evm_args
            let ty_str = quote! { #listener }.to_string();
            let this_listener = if ty_str.contains("EvmContractEventListener") {
                assert!(evm_args.is_some(), "EvmArgs must be passed");
                SingleListener {
                    listener,
                    evm_args,
                    listener_type: ListenerType::Evm,
                    event,
                    post_processor,
                    pre_processor,
                }
            } else {
                let listener_type = if ty_str.contains("TangleEventListener") {
                    ListenerType::Tangle
                } else {
                    ListenerType::Custom
                };

                SingleListener {
                    listener,
                    evm_args: None,
                    listener_type,
                    event,
                    post_processor,
                    pre_processor,
                }
            };

            listeners.push(this_listener);

            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>()?;
            }
        }

        if listeners.is_empty() {
            return Err(content.error("Expected at least one event listener"));
        }

        if listeners.len() > 1 {
            return Err(content.error("Only one event listener is currently supported"));
        }

        Ok(Self { listeners })
    }
}

#[derive(Debug)]
pub(crate) struct EvmArgs {
    instance: Option<Ident>,
    event: Option<Type>,
    event_converter: Option<Type>,
    callback: Option<Type>,
}

impl EventListenerArgs {
    fn get_event_listener(&self) -> &SingleListener {
        &self.listeners[0]
    }

    pub fn get_param_result_tokenstream(
        &self,
        fields: &[FieldType],
    ) -> Vec<proc_macro2::TokenStream> {
        let event_listener = self.get_event_listener();
        if fields.len() == 1 {
            let ident = format_ident!("job_result");
            match event_listener.listener_type {
                ListenerType::Evm => {
                    vec![quote! { let #ident = job_result; }]
                }

                ListenerType::Tangle => {
                    vec![crate::tangle::field_type_to_result_token(
                        &ident, &fields[0],
                    )]
                }

                ListenerType::Custom => {
                    vec![quote! { let #ident = job_result; }]
                }
            }
        } else {
            fields
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let ident = format_ident!("result_{i}");
                    match event_listener.listener_type {
                        ListenerType::Evm => {
                            quote! {
                                let #ident = job_result[#i];
                            }
                        }

                        ListenerType::Tangle => {
                            let s = crate::tangle::field_type_to_result_token(&ident, t);
                            quote! {
                                let #ident = job_result[#i];
                                #s
                            }
                        }

                        ListenerType::Custom => {
                            quote! {
                                let #ident = job_result[#i];
                            }
                        }
                    }
                })
                .collect::<Vec<_>>()
        }
    }

    pub fn get_param_name_tokenstream(
        &self,
        params: &[FieldType],
    ) -> Vec<proc_macro2::TokenStream> {
        params
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let ident = format_ident!("param{i}");
                let index = Index::from(i);
                match self.get_event_listener().listener_type {
                    ListenerType::Tangle => crate::tangle::field_type_to_param_token(&ident, t),
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

    /// Returns true if on EVM
    pub fn get_evm(&self) -> Option<&EvmArgs> {
        self.get_event_listener().evm_args.as_ref()
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

    /// Returns the Event Handler's Instance if on EigenLayer. Otherwise, returns None
    pub fn instance(&self) -> Option<Ident> {
        match self.get_evm() {
            Some(EvmArgs { instance, .. }) => instance.clone(),
            None => None,
        }
    }

    /// Returns the Event Handler's event if on EigenLayer. Otherwise, returns None
    pub fn event(&self) -> Option<Type> {
        match self.get_evm() {
            Some(EvmArgs { event, .. }) => event.clone(),
            None => None,
        }
    }

    /// Returns the Event Handler's Event Converter if on EigenLayer. Otherwise, returns None
    pub fn event_converter(&self) -> Option<Type> {
        match self.get_evm() {
            Some(EvmArgs {
                event_converter, ..
            }) => event_converter.clone(),
            None => None,
        }
    }

    /// Returns the Event Handler's Callback if on EigenLayer. Otherwise, returns None
    pub fn callback(&self) -> Option<Type> {
        match self.get_evm() {
            Some(EvmArgs { callback, .. }) => callback.clone(),
            None => None,
        }
    }
}

impl Parse for EvmArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);

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

        Ok(EvmArgs {
            instance,
            event,
            event_converter,
            callback,
        })
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
                gadget_sdk::error!("An Event Handler for {} has stopped running", stringify!(#struct_name));
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
