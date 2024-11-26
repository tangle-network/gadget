use quote::quote;
use syn::{parse_quote, DeriveInput};

use crate::cfg::FieldInfo;

/// Generate the `KeystoreContext` implementation for the given struct.
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

    // Create a new generics instance without the RwLock parameter
    let mut generics_without_rwlock = generics.clone();
    generics_without_rwlock
        .params
        .push(parse_quote!(RwLock: gadget_sdk::ext::lock_api::RawRwLock));

    let (impl_generics_without_rwlock, _, where_clause_without_rwlock) =
        generics_without_rwlock.split_for_impl();

    quote! {
        #[cfg(not(feature = "std"))]
        impl #impl_generics_without_rwlock gadget_sdk::contexts::KeystoreContext<RwLock> for #name #ty_generics #where_clause_without_rwlock {
            fn keystore(&self) -> Result<gadget_sdk::keystore::backend::GenericKeyStore<RwLock>, gadget_sdk::config::Error> {
                #field_access.keystore()
            }
        }

        #[cfg(feature = "std")]
        impl #impl_generics gadget_sdk::contexts::KeystoreContext<gadget_sdk::ext::parking_lot::RawRwLock> for #name #ty_generics #where_clause {
            fn keystore(&self) -> Result<gadget_sdk::keystore::backend::GenericKeyStore<gadget_sdk::ext::parking_lot::RawRwLock>, gadget_sdk::config::Error> {
                #field_access.keystore()
            }
        }
    }
}
