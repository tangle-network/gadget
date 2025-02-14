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
) -> proc_macro2::TokenStream {
    let field_access_config = match config_field {
        FieldInfo::Named(ident) => quote! { self.#ident },
        FieldInfo::Unnamed(index) => quote! { self.#index },
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let config_ty = quote! { ::blueprint_sdk::macros::ext::contexts::tangle::TangleClient };

    quote! {
        #[::blueprint_sdk::macros::ext::async_trait::async_trait]
        impl #impl_generics ::blueprint_sdk::macros::ext::contexts::tangle::TangleClientContext for #name #ty_generics #where_clause {
            async fn tangle_client(&self) -> std::result::Result<#config_ty, ::blueprint_sdk::macros::ext::clients::Error> {
                ::blueprint_sdk::macros::ext::contexts::tangle::TangleClientContext::tangle_client(&#field_access_config).await
            }
        }
    }
}
