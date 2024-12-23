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
) -> proc_macro2::TokenStream {
    let field_access = match config_field {
        FieldInfo::Named(ident) => quote! { self.#ident },
        FieldInfo::Unnamed(index) => quote! { self.#index },
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        #[::gadget_macros::ext::async_trait::async_trait]
        impl #impl_generics ::gadget_macros::ext::contexts::services::ServicesContext for #name #ty_generics #where_clause {
            async fn client(&self) -> ::gadget_macros::ext::contexts::services::ServicesClient<::gadget_macros::ext::tangle::tangle_subxt::subxt::PolkadotConfig> {
                todo!()
            }
        }
    }
}
