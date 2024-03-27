use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, GenericParam, Generics, ItemStruct};

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
                    Some(quote! { #ident, })
                } else {
                    Some(quote! { #ident: #bounds, })
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
pub fn protocol(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_struct = parse_macro_input!(input as ItemStruct);

    let struct_ident = &input_struct.ident;
    let new_struct = syn::Ident::new(&format!("_Internal{}", struct_ident), struct_ident.span());
    let struct_generics = &input_struct.generics;
    let where_bounds = &struct_generics
        .where_clause
        .clone()
        .map(|r| r.to_token_stream())
        .unwrap_or_default();
    let fields = &input_struct.fields;

    // Create a function that takes each field and clones it
    let clone_fields: proc_macro2::TokenStream = fields
        .iter()
        .map(|field| {
            let field_ident = field.ident.as_ref().unwrap();
            quote! { #field_ident: self.#field_ident.clone(), }
        })
        .collect();

    // Generate the generic parameters
    let generics_token_stream = generate_generic_params(struct_generics, None);
    let generics_token_stream_unique =
        generate_generic_params(struct_generics, Some(&["C", "B", "BE"]));
    let generics_token_stream_unique_with_bounds =
        generate_generic_params_with_bounds(struct_generics, Some(&["C", "B", "BE"]));
    let generic_token_stream_with_bounds =
        generate_generic_params_with_bounds(struct_generics, None);
    // Create the implementation
    let generated = quote! {
        #input_struct

        pub struct #new_struct<C: gadget_common::config::ClientWithApi + 'static, #generics_token_stream_unique_with_bounds>
            #where_bounds {
            pub network: Option<<#struct_ident<#generics_token_stream> as gadget_common::config::NetworkAndProtocolSetup>::Network>,
            pub protocol: Option<<#struct_ident<#generics_token_stream> as gadget_common::config::NetworkAndProtocolSetup>::Protocol>,
            pub client: Option<C>,
            pub params: #struct_ident<#generics_token_stream>,
            pub pallet_tx: Arc<dyn gadget_common::client::PalletSubmitter>,
            pub logger: gadget_common::config::DebugLogger,
            pub prometheus_config: gadget_common::prometheus::PrometheusConfig,
        }

        impl<C: gadget_common::config::ClientWithApi + 'static, #generics_token_stream_unique_with_bounds> gadget_common::config::ProtocolConfig for #new_struct <C, #generics_token_stream_unique>
            #where_bounds
        {
            type Network = <Self::ProtocolSpecificConfiguration as gadget_common::config::NetworkAndProtocolSetup>::Network;
            type Protocol = <Self::ProtocolSpecificConfiguration as gadget_common::config::NetworkAndProtocolSetup>::Protocol;
            type ProtocolSpecificConfiguration = #struct_ident <#generics_token_stream>;

            fn new(network: Self::Network, client: <Self::ProtocolSpecificConfiguration as gadget_common::config::NetworkAndProtocolSetup>::Client, protocol: Self::Protocol, params: Self::ProtocolSpecificConfiguration, pallet_tx: Arc<dyn gadget_common::client::PalletSubmitter>, logger: DebugLogger, prometheus_config: gadget_common::prometheus::PrometheusConfig) -> Self {
                Self {
                    network: Some(network),
                    client: Some(client),
                    protocol: Some(protocol),
                    params,
                    pallet_tx,
                    logger,
                    prometheus_config,
                }
            }

            fn take_network(&mut self) -> Self::Network {
                self.network.take().expect("Network not set")
            }

            fn take_protocol(&mut self) -> Self::Protocol {
                self.protocol.take().expect("Protocol not set")
            }

            fn take_client(&mut self) -> <Self::ProtocolSpecificConfiguration as gadget_common::config::NetworkAndProtocolSetup>::Client {
                self.client.take().expect("Client not set")
            }

            fn prometheus_config(&self) -> gadget_common::config::PrometheusConfig {
                self.prometheus_config.clone()
            }

            fn params(&self) -> &Self::ProtocolSpecificConfiguration {
                &self.params
            }
        }

        impl<C: gadget_common::config::ClientWithApi + 'static, #generics_token_stream_unique_with_bounds> #struct_ident <#generics_token_stream>
            #where_bounds
        {
            pub fn setup(self) -> #new_struct <C, #generics_token_stream_unique> {
                let pallet_tx = <Self as gadget_common::config::NetworkAndProtocolSetup>::pallet_tx(&self);
                let logger = <Self as gadget_common::config::NetworkAndProtocolSetup>::logger(&self);
                let prometheus_config = self.prometheus_config.clone();

                #new_struct {
                    network: None,
                    protocol: None,
                    client: None,
                    params: self,
                    pallet_tx,
                    logger,
                    prometheus_config,
                }
            }

            pub async fn execute(self) -> Result<(), gadget_io::Error> {
                use gadget_common::config::ProtocolConfig;
                self.setup().build().await?.run().await
            }
        }

        impl<#generic_token_stream_with_bounds> Clone for #struct_ident <#generics_token_stream> #where_bounds {
            fn clone(&self) -> Self {
                Self {
                    #clone_fields
                }
            }
        }
    };

    generated.into()
}
