use quote::quote;
use syn::DeriveInput;

use crate::cfg::FieldInfo;

/// Generate the `MPCContext` implementation for the given struct.
#[allow(clippy::too_many_lines)]
pub fn generate_context_impl(
    DeriveInput {
        ident: name,
        generics,
        ..
    }: DeriveInput,
    config_field: FieldInfo,
) -> proc_macro2::TokenStream {
    let _field_access = match config_field {
        FieldInfo::Named(ident) => quote! { self.#ident },
        FieldInfo::Unnamed(index) => quote! { self.#index },
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        #[gadget_sdk::async_trait::async_trait]
        impl #impl_generics gadget_sdk::contexts::MPCContext for #name #ty_generics #where_clause {
            /// Returns a reference to the configuration
            #[inline]
            fn config(&self) -> &gadget_sdk::config::StdGadgetConfiguration {
                &self.config
            }

            /// Returns the network protocol identifier for this context
            #[inline]
            fn network_protocol(&self) -> String {
                let name = stringify!(#name).to_string();
                format!("/{}/1.0.0", name.to_lowercase())
            }
        }
    }
}
