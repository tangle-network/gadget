use quote::quote;
use syn::DeriveInput;

use crate::cfg::FieldInfo;

/// Generate the `TangleClientContext` implementation for the given struct.
pub fn generate_context_impl(
    DeriveInput {
        ident: name,
        generics,
        ..
    }: DeriveInput,
    config_field: FieldInfo,
    call_id_field: FieldInfo,
) -> proc_macro2::TokenStream {
    let field_access_config = match config_field {
        FieldInfo::Named(ident) => quote! { self.#ident },
        FieldInfo::Unnamed(index) => quote! { self.#index },
    };

    let field_access_call_id = match call_id_field {
        FieldInfo::Named(ident) => quote! { self.#ident },
        FieldInfo::Unnamed(index) => quote! { self.#index },
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let config_ty = quote! { ::gadget_macros::ext::contexts::tangle::TangleClient };

    quote! {
        #[::gadget_macros::ext::async_trait::async_trait]
        impl #impl_generics ::gadget_macros::ext::contexts::tangle::TangleClientContext for #name #ty_generics #where_clause {
            async fn tangle_client(&self) -> Result<#config_ty, ::gadget_macros::ext::tangle::tangle_subxt::subxt::Error> {
                use ::gadget_macros::ext::tangle::tangle_subxt::subxt;

                type Config = #config_ty;
                static CLIENT: std::sync::OnceLock<Config> = std::sync::OnceLock::new();
                match CLIENT.get() {
                    Some(client) => Ok(client.clone()),
                    None => {
                        let rpc_url = #field_access_config.ws_rpc_endpoint.as_str();
                        let client = subxt::OnlineClient::from_url(rpc_url).await?;
                        CLIENT.set(client.clone()).map(|_| client).map_err(|_| {
                            subxt::Error::Io(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Failed to set client",
                            ))
                        })
                    }
                }
            }

            fn get_call_id(&mut self) -> &mut Option<u64> {
                &mut #field_access_call_id
            }
        }
    }
}
