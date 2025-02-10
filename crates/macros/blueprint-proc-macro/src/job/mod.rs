mod args;
pub(crate) use args::{EventListenerArgs, JobArgs, ListenerType};

#[cfg(feature = "evm")]
mod evm;
#[cfg(feature = "evm")]
use evm::{generate_evm_specific_impl, get_evm_instance_data, get_evm_job_processor_wrapper};

#[cfg(feature = "tangle")]
mod tangle;
#[cfg(feature = "tangle")]
use tangle::{generate_tangle_specific_impl, get_tangle_job_processor_wrapper};

use crate::shared::{self, get_return_type_wrapper, pascal_case, type_to_field_type, MacroExt};

use gadget_blueprint_proc_macro_core::{FieldType, JobDefinition, JobMetadata};
use gadget_std::str::FromStr;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, LitInt, Token, Type};

/// Defines custom keywords for defining Job arguments
mod kw {
    syn::custom_keyword!(id);
    syn::custom_keyword!(params);
    syn::custom_keyword!(result);
    syn::custom_keyword!(evm);
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

struct JobDef {
    args: JobArgs,
    input: ItemFn,
}

impl JobDef {
    const SUFFIX: &str = "EventHandler";

    fn generate(self) -> syn::Result<TokenStream> {
        let param_map = shared::param_types(&self.input.sig)?;
        let params_type = declared_params_to_field_types(&self.args.params, &param_map)?;

        let result = get_return_type(&self.input);
        let result_type = self.args.result_to_field_types(&result)?;

        let job_id = &self.args.id;
        let job_const_block =
            generate_job_const_block(&self.input, params_type, result_type, job_id)?;

        if self.args.skip_codegen {
            let input = &self.input;
            return Ok(quote! {
                #job_const_block

                #[allow(unused_variables)]
                #input
            });
        }

        let (event_listener_gen, event_listener_calls) = generate_event_workflow_tokenstream(
            &self.input,
            Self::SUFFIX,
            &self.args.event_listener,
            &param_map,
            &self.args.params,
        )?;

        // Generate Event Workflow
        let additional_specific_logic = generate_specialized_logic(
            &self.input,
            &self.args.event_listener,
            Self::SUFFIX,
            &param_map,
            &self.args.params,
        )?;

        let autogen_struct = generate_autogen_struct(
            &self.input,
            &self.args.event_listener,
            &self.args.params,
            &param_map,
            Self::SUFFIX,
            &event_listener_calls,
        )?;

        // Generates final TokenStream that will be returned
        let input = &self.input;
        Ok(quote! {
            #job_const_block

            #autogen_struct

            #(#event_listener_gen)*

            #[allow(unused_variables)]
            #input

            #additional_specific_logic
        })
    }
}

/// Job Macro implementation
pub(crate) fn job_impl(args: JobArgs, input: ItemFn) -> syn::Result<TokenStream> {
    let def = JobDef { args, input };

    def.generate()
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
) -> syn::Result<TokenStream> {
    let (fn_name_string, job_def_name, job_id_name) = get_job_id_field_name(input);
    // Creates Job Definition using input parameters
    let job_id_as_u64 = u64::from_str(&job_id.to_string()).map_err(|err| {
        syn::Error::new_spanned(job_id, format!("Failed to convert job id to u64: {err}"))
    })?;
    let job_def = JobDefinition {
        job_id: job_id_as_u64,
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
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_event_workflow_tokenstream(
    input: &ItemFn,
    suffix: &str,
    event_listeners: &EventListenerArgs,
    param_types: &IndexMap<Ident, Type>,
    params: &[Ident],
) -> syn::Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let return_type = get_return_type(input);

    let (mut event_handler_args, event_handler_arg_types) =
        get_event_handler_args(param_types, params)?;
    let (_, _, struct_name) = generate_fn_name_and_struct(input, suffix);
    #[cfg(feature = "tangle")]
    let (fn_name_string, _job_def_name, job_id_name) = get_job_id_field_name(input);
    #[cfg(not(feature = "tangle"))]
    let (fn_name_string, _job_def_name, _job_id_name) = get_job_id_field_name(input);

    // Generate Event Listener
    let mut event_listener_gen = vec![];
    let mut event_listener_calls = vec![];

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

        let (ctx_pos_in_ordered_inputs, mut ordered_inputs) = match listener_meta.listener_type {
            #[cfg(feature = "evm")]
            ListenerType::Evm => get_fn_call_ordered(param_types, params, is_raw)?,
            #[cfg(feature = "tangle")]
            ListenerType::Tangle => get_fn_call_ordered(param_types, params, is_raw)?,
            ListenerType::Custom => get_fn_call_ordered(param_types, params, is_raw)?,
        };

        let asyncness = get_asyncness(input);

        // TODO: task 001: find better way to identify which ident is the raw event
        // for now, we assume the raw event is always listed first
        if is_raw {
            let _ = event_handler_args.remove(0);
        }

        let (context_field, context_ty) = event_handler_args
            .first()
            .map(|field_in_self| {
                // If is_raw, assume the actual context is the second param
                (
                    quote! { handler. #field_in_self .clone() },
                    event_handler_arg_types[0],
                )
            })
            .ok_or_else(|| {
                syn::Error::new(
                    Span::call_site(),
                    "Must specify a context (field_in_self_getter)",
                )
            })?;

        let autogen_struct_name = quote! { #struct_name };

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
            quote! {
                |evt| async move {
                    Ok(Some(evt))
                }
            }
        };

        // The job_processor is just the job function. Since it may contain multiple params, we need a new function to call it.
        let job_processor_wrapper = match listener_meta.listener_type {
            #[cfg(feature = "tangle")]
            ListenerType::Tangle => get_tangle_job_processor_wrapper(
                params,
                param_types,
                event_listeners,
                &mut ordered_inputs,
                fn_name_ident,
                &asyncness,
                &return_type,
                context_ty,
                ctx_pos_in_ordered_inputs,
            )?,

            #[cfg(feature = "evm")]
            ListenerType::Evm => get_evm_job_processor_wrapper(
                params,
                param_types,
                event_listeners,
                &mut ordered_inputs,
                fn_name_ident,
                &asyncness,
                &return_type,
                ctx_pos_in_ordered_inputs,
            )?,

            ListenerType::Custom => {
                let job_processor_call_return = get_return_type_wrapper(&return_type, None);

                quote! {
                    move |param0| async move {
                        let res = #fn_name_ident (#(#ordered_inputs),*) #asyncness;
                        #job_processor_call_return
                    }
                }
            }
        };

        let post_processor_function = if let Some(postprocessor) = &listener_meta.post_processor {
            match listener_meta.listener_type {
                #[cfg(feature = "tangle")]
                ListenerType::Tangle => {
                    // TODO: Double clone on client and signer
                    quote! {
                        move |(mut context, job_result)| {
                            let client = client.clone();
                            let signer = signer.clone();
                            async move {
                                let call_id = ::blueprint_sdk::macros::ext::contexts::services::ServicesContext::get_call_id(&mut context).expect("Tangle call ID was not injected into context");
                                let tangle_job_result = ::blueprint_sdk::macros::ext::event_listeners::tangle::events::TangleResult::<_> {
                                    results: job_result,
                                    service_id,
                                    call_id,
                                    client,
                                    signer,
                                };

                                #postprocessor(tangle_job_result).await
                            }
                        }
                    }
                }

                #[cfg(feature = "evm")]
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
            #[cfg(feature = "tangle")]
            ListenerType::Tangle => {
                quote! {
                    let context: ::blueprint_sdk::macros::ext::event_listeners::tangle::events::TangleListenerInput<#context_ty> = ::blueprint_sdk::macros::ext::event_listeners::tangle::events::TangleListenerInput {
                        client: handler.client.subxt_client().clone(),
                        signer: handler.signer.clone(),
                        job_id: #job_id_name,
                        service_id: handler.service_id,
                        context: #context_field,
                    };

                    let client = context.client.clone();
                    let signer = context.signer.clone();
                    let service_id = context.service_id;
                }
            }

            #[cfg(feature = "evm")]
            ListenerType::Evm => {
                quote! {
                    let context = #context_field;
                }
            }

            ListenerType::Custom => {
                quote! { let context = #context_field; }
            }
        };

        let context_to_listener = match listener_meta.listener_type {
            #[cfg(feature = "tangle")]
            ListenerType::Tangle => quote! { &context },
            #[cfg(feature = "evm")]
            ListenerType::Evm => quote! { &handler },
            ListenerType::Custom => quote! { &context },
        };

        let event_listener_creator_param = match listener_meta.listener_type {
            #[cfg(feature = "tangle")]
            ListenerType::Tangle => quote! { _ },
            #[cfg(feature = "evm")]
            ListenerType::Evm => quote! { #autogen_struct_name },
            ListenerType::Custom => quote! { _ },
        };

        let wrapper_type_params = match listener_meta.listener_type {
            #[cfg(feature = "evm")]
            ListenerType::Evm => quote! { ::<_, _, #autogen_struct_name> },
            _ => quote! {},
        };

        let next_listener = quote! {
            async fn #listener_function_name (handler: &#autogen_struct_name) -> Option<::blueprint_sdk::macros::ext::tokio::sync::oneshot::Receiver<Result<(), Box<dyn ::core::error::Error + Send>>>> {
                static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                if !ONCE.fetch_or(true, std::sync::atomic::Ordering::Relaxed) {
                    //::blueprint_sdk::logging::warn!("(Test mode?) Duplicated call for event listener {}", stringify!(#struct_name));
                }

                let (tx, rx) = ::blueprint_sdk::macros::ext::tokio::sync::oneshot::channel();

                #context_declaration
                let job_processor = #job_processor_wrapper;

                let listener = <#listener as ::blueprint_sdk::macros::ext::event_listeners::core::EventListener<_, _, #event_listener_creator_param>>::new(#context_to_listener).await.expect("Failed to create event listener");
                let mut event_workflow = ::blueprint_sdk::macros::ext::event_listeners::core::executor::EventFlowWrapper::new(
                    context,
                    listener,
                    #pre_processor_function,
                    job_processor,
                    #post_processor_function,
                );

                let task = async move {
                    let res = ::blueprint_sdk::macros::ext::event_listeners::core::executor::EventFlowExecutor #wrapper_type_params::event_loop(&mut event_workflow).await.map_err(|e| Box::new(e) as Box<dyn ::core::error::Error + Send>);
                    let _ = tx.send(res);
                };
                ::blueprint_sdk::macros::ext::tokio::task::spawn(task);
                return Some(rx)
            }
        };

        event_listener_gen.push(next_listener);
    }

    Ok((event_listener_gen, event_listener_calls))
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
    event_listener_calls: &[TokenStream],
) -> syn::Result<TokenStream> {
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

    let mut required_fields: Vec<TokenStream> = vec![];

    // Even if multiple tangle listeners, we only need this once
    #[cfg(feature = "tangle")]
    if event_listener_args.has_tangle() {
        required_fields.push(quote! {
            pub service_id: u64,
            pub signer: ::blueprint_sdk::macros::ext::crypto::tangle_pair_signer::TanglePairSigner<::blueprint_sdk::macros::ext::crypto::tangle_pair_signer::sp_core::sr25519::Pair>,
            pub client: ::blueprint_sdk::macros::ext::clients::tangle::client::TangleClient,
        })
    }

    // Even if multiple evm listeners, we only need this once
    #[cfg(feature = "evm")]
    if event_listener_args.has_evm() {
        let (_, _, _, instance_name) = get_evm_instance_data(event_listener_args)?;

        required_fields.push(quote! {
            pub contract: #instance_name,
            pub contract_instance: std::sync::OnceLock<::blueprint_sdk::macros::ext::event_listeners::evm::AlloyContractInstance>,
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

        #[::blueprint_sdk::macros::ext::async_trait::async_trait]
        impl ::blueprint_sdk::macros::ext::event_listeners::core::InitializableEventHandler for #struct_name {
            async fn init_event_handler(
                &self,
            ) -> Option<
                ::blueprint_sdk::macros::ext::tokio::sync::oneshot::Receiver<
                    Result<(), Box<dyn ::core::error::Error + Send>>
                >
            > {
                #(#event_listener_calls)*
                #combined_event_listener
            }
        }
    })
}

pub fn get_fn_call_ordered(
    param_types: &IndexMap<Ident, Type>,
    params_from_job_args: &[Ident],
    is_raw: bool,
) -> syn::Result<(usize, Vec<TokenStream>)> {
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
    let this = quote! { handler };
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

fn get_asyncness(input: &ItemFn) -> TokenStream {
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
) -> syn::Result<TokenStream> {
    let (_fn_name, _fn_name_string, struct_name) = generate_fn_name_and_struct(input, suffix);

    match event_listener_args.get_event_listener().listener_type {
        #[cfg(feature = "evm")]
        ListenerType::Evm => {
            generate_evm_specific_impl(&struct_name, event_listener_args, param_map, job_params)
        }

        #[cfg(feature = "tangle")]
        ListenerType::Tangle => {
            generate_tangle_specific_impl(&struct_name, param_map, job_params, event_listener_args)
        }

        ListenerType::Custom => Ok(TokenStream::default()),
    }
}

#[derive(Debug, Clone)]
pub struct ParameterType {
    pub ty: FieldType,
    #[allow(dead_code)]
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

pub(crate) fn generate_combined_event_listener_selector(struct_name: &Ident) -> TokenStream {
    quote! {
        let (tx, rx) = ::blueprint_sdk::macros::ext::tokio::sync::oneshot::channel();
        let task = async move {
            let mut futures = ::blueprint_sdk::macros::ext::futures::stream::FuturesUnordered::new();
            for listener in listeners {
                futures.push(listener);
            }
            if let Some(res) = ::blueprint_sdk::macros::ext::futures::stream::StreamExt::next(&mut futures).await {
                //::blueprint_sdk::macros::ext::logging::warn!("An Event Handler for {} has stopped running", stringify!(#struct_name));
                let res = match res {
                    Ok(res) => {
                        res
                    },
                    Err(e) => {
                        Err(Box::new(::blueprint_sdk::macros::ext::event_listeners::core::Error::<
                                ::blueprint_sdk::macros::ext::event_listeners::core::error::Unit
                            >::Other(format!("Error in Event Handler for {}: {e:?}", stringify!(#struct_name)))) as Box<dyn ::core::error::Error + Send>)
                    }
                };

                tx.send(res).unwrap();
            }
        };
        let _ = ::blueprint_sdk::macros::ext::tokio::spawn(task);
        Some(rx)
    }
}
