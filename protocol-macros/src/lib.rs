use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Expr, GenericParam, Generics, ItemStruct};

struct MacroInput {
    protocol_name: Expr,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            protocol_name: input.parse()?,
        })
    }
}

fn generate_generic_params_with_bounds(
    generics: &Generics,
    excluded: Option<&[&str]>,
) -> proc_macro2::TokenStream {
    generics
        .params
        .iter()
        .filter_map(|param| {
            if let GenericParam::Type(ty) = param {
                if let Some(excluded) = excluded {
                    if excluded.contains(&ty.ident.to_string().as_str()) {
                        return None;
                    }
                }
                let ident = ty.ident.clone();
                let bounds = ty.bounds.clone();
                if bounds.is_empty() {
                    Some(quote! { #ident })
                } else {
                    Some(quote! { #ident: #bounds })
                }
            } else {
                None
            }
        })
        .collect()
}

fn generate_generic_params(
    generics: &Generics,
    excluded: Option<&[&str]>,
) -> proc_macro2::TokenStream {
    let generic_idents: Vec<_> = generics
        .params
        .iter()
        .filter_map(|param| {
            if let GenericParam::Type(ty) = param {
                if let Some(excluded) = excluded {
                    if excluded.contains(&ty.ident.to_string().as_str()) {
                        return None;
                    }
                }
                Some(&ty.ident)
            } else {
                None
            }
        })
        .collect();

    // Create a TokenStream from the generic identifiers
    let generics_ts: proc_macro2::TokenStream = generic_idents
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, ident)| {
            if i < generic_idents.len() - 1 {
                quote! { #ident , }
            } else {
                quote! { #ident }
            }
        })
        .collect();

    generics_ts
}

#[proc_macro_attribute]
pub fn define_protocol(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_struct = parse_macro_input!(input as ItemStruct);
    let args_parsed = parse_macro_input!(args as MacroInput);
    let new_struct = args_parsed.protocol_name.to_token_stream();
    let struct_ident = &input_struct.ident;
    let struct_generics = &input_struct.generics;
    let where_bounds = &struct_generics
        .where_clause
        .clone()
        .map(|r| r.to_token_stream())
        .unwrap_or_default();

    // Generate the generic parameters
    let generics_token_stream = generate_generic_params(struct_generics, None);
    let generics_token_stream_unique =
        generate_generic_params(struct_generics, Some(&["N", "C", "B", "BE"]));
    let generics_token_stream_unique_with_bounds =
        generate_generic_params_with_bounds(struct_generics, Some(&["N", "C", "B", "BE"]));
    // Create the implementation
    let generated = quote! {
        #input_struct

        pub struct #new_struct<N: gadget_common::config::Network, C: gadget_common::config::ClientWithApi<B, BE>, B: gadget_common::config::Block, BE: gadget_common::config::Backend<B>, #generics_token_stream_unique_with_bounds>
            #where_bounds {
            pub network: Option<<#struct_ident<#generics_token_stream> as gadget_common::config::NetworkAndProtocolSetup>::Network>,
            pub protocol: Option<<#struct_ident<#generics_token_stream> as gadget_common::config::NetworkAndProtocolSetup>::Protocol>,
            pub client: Option<C>,
            pub params: #struct_ident<#generics_token_stream>,
            pallet_tx: Arc<dyn gadget_common::client::PalletSubmitter>,
            logger: gadget_common::config::DebugLogger,
            _pd: std::marker::PhantomData<(B, BE)>,
        }

        impl<N: gadget_common::config::Network, C: gadget_common::config::ClientWithApi<B, BE>, B: gadget_common::config::Block, BE: gadget_common::config::Backend<B> + 'static, #generics_token_stream_unique_with_bounds> gadget_common::config::ProtocolConfig for #new_struct<N, C, B, BE, #generics_token_stream_unique>
            #where_bounds,
        {
            type Network = <Self::ProtocolSpecificConfiguration as gadget_common::config::NetworkAndProtocolSetup>::Network;
            type Block = B;
            type Backend = BE;
            type Protocol = <Self::ProtocolSpecificConfiguration as gadget_common::config::NetworkAndProtocolSetup>::Protocol;
            type Client = C;
            type ProtocolSpecificConfiguration = #struct_ident<#generics_token_stream>;

            fn new(network: Self::Network, client: Self::Client, protocol: Self::Protocol, params: Self::ProtocolSpecificConfiguration, pallet_tx: Arc<dyn gadget_common::client::PalletSubmitter>, logger: DebugLogger) -> Self {
                Self {
                    network: Some(network),
                    client: Some(client),
                    protocol: Some(protocol),
                    params,
                    pallet_tx,
                    logger,
                    _pd: std::marker::PhantomData,
                }
            }

            fn take_network(&mut self) -> Self::Network {
                self.network.take().expect("Network not set")
            }

            fn take_protocol(&mut self) -> Self::Protocol {
                self.protocol.take().expect("Protocol not set")
            }

            fn take_client(&mut self) -> Self::Client {
                self.client.take().expect("Client not set")
            }

            fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
                self.pallet_tx.clone()
            }

            fn logger(&self) -> DebugLogger {
                self.logger.clone()
            }

            fn client_inner(&self) -> Self::Client {
                self.client.as_ref().expect("Client not set").clone()
            }

            fn params(&self) -> &Self::ProtocolSpecificConfiguration {
                &self.params
            }
        }
    };

    generated.into()
}
