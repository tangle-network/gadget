use quote::quote;
use syn::DeriveInput;

use crate::cfg::FieldInfo;

/// Generate the `ServicesContext` implementation for the given struct.
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

    let config_ty = quote! {
        ::blueprint_sdk::contexts::services::TangleServicesClient<::blueprint_sdk::tangle_subxt::subxt::PolkadotConfig>
    };

    quote! {
        #[::blueprint_sdk::async_trait::async_trait]
        impl #impl_generics ::blueprint_sdk::contexts::services::ServicesContext for #name #ty_generics #where_clause {
            fn get_call_id(&mut self) -> &mut Option<u64> {
                &mut #field_access_call_id
            }

            async fn services_client(&self) -> #config_ty {
                let rpc_client = ::blueprint_sdk::tangle_subxt::subxt::OnlineClient::from_url(
                    &#field_access_config.http_rpc_endpoint
                )
                .await
                .expect("Failed to create RPC client");

                ::blueprint_sdk::contexts::services::TangleServicesClient::new(rpc_client)
            }
        }
    }
}
