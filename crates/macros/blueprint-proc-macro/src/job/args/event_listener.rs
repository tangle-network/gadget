#[cfg(feature = "evm")]
use crate::job::evm::EvmArgs;
use crate::job::ParameterType;
#[cfg(feature = "evm")]
use proc_macro2::Ident;
#[cfg(feature = "tangle")]
use quote::quote_spanned;
use quote::{format_ident, quote};
#[cfg(feature = "tangle")]
use std::str::FromStr;
use syn::parse::{Parse, ParseBuffer, ParseStream};
#[cfg(any(not(feature = "evm"), not(feature = "tangle")))]
use syn::spanned::Spanned;
use syn::{Index, Token, Type};

const EVM_EVENT_LISTENER_TAG: &str = "EvmContractEventListener";
const TANGLE_EVENT_LISTENER_TAG: &str = "TangleEventListener";

/// Defines custom keywords for defining Job arguments
mod kw {
    syn::custom_keyword!(instance);
    syn::custom_keyword!(listener);
    syn::custom_keyword!(event_listener);
    syn::custom_keyword!(pre_processor);
    syn::custom_keyword!(post_processor);
    syn::custom_keyword!(abi);
    syn::custom_keyword!(skip_codegen);
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ListenerType {
    #[cfg(feature = "evm")]
    Evm,
    #[cfg(feature = "tangle")]
    Tangle,
    Custom,
}

pub(crate) struct SingleListener {
    pub listener: Type,
    #[cfg(feature = "evm")]
    pub evm_args: Option<EvmArgs>,
    pub listener_type: ListenerType,
    pub post_processor: Option<Type>,
    pub pre_processor: Option<Type>,
}

impl SingleListener {
    #[cfg(feature = "tangle")]
    pub fn is_raw(&self) -> bool {
        self.pre_processor.is_none() && matches!(self.listener_type, ListenerType::Tangle)
    }

    #[cfg(not(feature = "tangle"))]
    pub fn is_raw(&self) -> bool {
        false
    }
}

/// `#[job(event_listener(MyCustomListener, MyCustomListener2)]`
/// Accepts an optional argument that specifies the event listener to use that implements EventListener
pub(crate) struct EventListenerArgs {
    pub(crate) listeners: Vec<SingleListener>,
}

impl Parse for EventListenerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<kw::event_listener>()?;
        let content;
        syn::parenthesized!(content in input);

        let mut listener = None;
        let mut pre_processor = None;
        let mut post_processor = None;
        #[allow(unused_mut)]
        let mut is_evm = false;
        // EVM specific
        #[cfg(feature = "evm")]
        let mut instance: Option<Ident> = None;
        #[cfg(feature = "evm")]
        let mut abi: Option<Type> = None;

        while !content.is_empty() {
            if content.peek(kw::listener) {
                let listener_found =
                    extract_x_equals_y::<kw::listener, Type>(&content, true, "listener")?
                        .ok_or_else(|| content.error("Expected `listener` field"))?;

                let ty_str = quote! { #listener_found }.to_string();

                if ty_str.contains(EVM_EVENT_LISTENER_TAG) {
                    #[cfg(not(feature = "evm"))]
                    return Err(syn::Error::new(
                        listener.span(),
                        "EVM event listeners require the `evm` feature to be enabled",
                    ));

                    #[cfg(feature = "evm")]
                    {
                        is_evm = true;
                    }
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
                #[cfg(not(feature = "evm"))]
                return Err(syn::Error::new(
                    listener.span(),
                    "EVM event listeners require the `evm` feature to be enabled",
                ));

                #[cfg(feature = "evm")]
                {
                    let _ = content.parse::<kw::instance>()?;
                    let _ = content.parse::<Token![=]>()?;
                    instance = Some(content.parse::<Ident>()?);
                }
            } else if content.peek(kw::abi) {
                #[cfg(not(feature = "evm"))]
                return Err(syn::Error::new(
                    listener.span(),
                    "EVM event listeners require the `evm` feature to be enabled",
                ));

                #[cfg(feature = "evm")]
                {
                    let _ = content.parse::<kw::abi>()?;
                    let _ = content.parse::<Token![=]>()?;
                    abi = Some(content.parse::<Type>()?);
                }
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
            #[cfg(not(feature = "evm"))]
            return Err(syn::Error::new(
                listener.span(),
                "EVM event listeners require the `evm` feature to be enabled",
            ));

            #[cfg(feature = "evm")]
            {
                let listener_type = ListenerType::Evm;
                if instance.is_none() {
                    return Err(
                        content.error("Expected `instance` argument for EVM event listener")
                    );
                }

                if abi.is_none() {
                    return Err(content.error("Expected `abi` argument for EVM event listener"));
                }

                SingleListener {
                    listener,
                    #[cfg(feature = "evm")]
                    evm_args: Some(EvmArgs { instance, abi }),
                    listener_type,
                    post_processor,
                    pre_processor,
                }
            }
        } else {
            let listener_type = if ty_str.contains(TANGLE_EVENT_LISTENER_TAG) {
                #[cfg(not(feature = "tangle"))]
                return Err(syn::Error::new(
                    listener.span(),
                    "Tangle event listeners require the `tangle` feature to be enabled",
                ));

                #[cfg(feature = "tangle")]
                ListenerType::Tangle
            } else {
                ListenerType::Custom
            };

            SingleListener {
                listener,
                #[cfg(feature = "evm")]
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
					#[cfg(feature = "tangle")]
					ListenerType::Tangle => {
						let ty_token_stream = proc_macro2::TokenStream::from_str(&param_ty.ty.as_rust_type()).expect("should be valid");
						let ty_tokens = quote_spanned! {param_ty.span.expect("should always be available")=>
                            #ty_token_stream
                        };
						quote! {
                            let __arg = args.next().expect("parameter count checked before");
                            let Ok(#ident) = gadget_macros::ext::blueprint_serde::from_field::<#ty_tokens>(__arg) else {
                                return Err(gadget_macros::ext::event_listeners::core::Error::BadArgumentDecoding(format!("Failed to decode the field `{}` to `{}`", stringify!(#ident), stringify!(#ty_tokens))));
                            };
                        }
					}

					#[cfg(feature = "evm")]
					ListenerType::Evm => {
                        let _ = param_ty;
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

    #[cfg(feature = "tangle")]
    pub fn has_tangle(&self) -> bool {
        self.listeners
            .iter()
            .any(|r| r.listener_type == ListenerType::Tangle)
    }

    #[cfg(feature = "evm")]
    pub fn has_evm(&self) -> bool {
        self.listeners
            .iter()
            .any(|r| r.listener_type == ListenerType::Evm)
    }

    /// Returns the Event Handler's Contract Instance on the EVM.
    #[cfg(feature = "evm")]
    pub fn instance(&self) -> Option<Ident> {
        match self.get_event_listener().evm_args.as_ref() {
            Some(EvmArgs { instance, .. }) => instance.clone(),
            None => None,
        }
    }
}
