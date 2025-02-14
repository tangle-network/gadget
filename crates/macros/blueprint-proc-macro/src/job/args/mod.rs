pub mod event_listener;

pub use event_listener::{EventListenerArgs, ListenerType};

use super::{Results, ResultsKind};
use crate::shared::MacroExt;
use proc_macro2::Ident;
use std::collections::HashSet;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{LitInt, Token};

/// Defines custom keywords for defining Job arguments
mod kw {
    syn::custom_keyword!(id);
    syn::custom_keyword!(params);
    syn::custom_keyword!(result);
    syn::custom_keyword!(event_listener);
    syn::custom_keyword!(instance);
    syn::custom_keyword!(skip_codegen);
}

/// `JobArgs` type to handle parsing of attributes
pub(crate) struct JobArgs {
    /// Unique identifier for the job in the blueprint
    /// `#[job(id = 1)]`
    pub id: LitInt,
    /// List of parameters for the job, in order.
    /// `#[job(params(a, b, c))]`
    pub params: Vec<Ident>,
    /// List of return types for the job, could be inferred from the function return type.
    /// `#[job(result(u32, u64))]`
    /// `#[job(result(_))]`
    pub result: ResultsKind,
    /// Optional: Event listener type for the job
    /// `#[job(event_listener(MyCustomListener))]`
    pub event_listener: EventListenerArgs,
    /// Optional: Skip code generation for this job.
    /// `#[job(skip_codegen)]`
    /// this is useful if the developer want to impl a custom event handler
    /// for this job.
    pub skip_codegen: bool,
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
