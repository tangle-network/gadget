#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    unsafe_code,
    unstable_features,
    unused_qualifications,
    missing_docs,
    unused_results,
    clippy::exhaustive_enums
)]
//! Proc-macros for deriving Context Extensions from [`gadget-sdk`](https://crates.io/crates/gadget-sdk) crate.
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, Data, DeriveInput, Error, Fields, Ident, Index, Result};

/// Derive macro for generating Context Extensions trait implementation for `KeystoreContext`.
#[proc_macro_derive(KeystoreContext, attributes(config))]
pub fn derive_keystore_context(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let result = find_config_field(&input.ident, &input.data)
        .map(|config_field| generate_keystore_context_impl(input, config_field));

    match result {
        Ok(expanded) => TokenStream::from(expanded),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

enum FieldInfo {
    Named(Ident),
    Unnamed(Index),
}

fn find_config_field(input_ident: &Ident, data: &Data) -> Result<FieldInfo> {
    match data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => {
                for field in &fields.named {
                    if field
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident("config"))
                    {
                        return field.ident.clone().map(FieldInfo::Named).ok_or_else(|| {
                            Error::new_spanned(
                                field,
                                "Expected named field with #[config] attribute",
                            )
                        });
                    }
                }
                Err(Error::new_spanned(
                    input_ident,
                    "No field with #[config] attribute found, please add #[config] to the field that holds the `gadget_sdk::config::GadgetConfiguration`",
                ))
            }
            Fields::Unnamed(fields) => {
                for (i, field) in fields.unnamed.iter().enumerate() {
                    if field
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident("config"))
                    {
                        return Ok(FieldInfo::Unnamed(Index::from(i)));
                    }
                }
                Err(Error::new_spanned(
                    input_ident,
                    "No field with #[config] attribute found, please add #[config] to the field that holds the `gadget_sdk::config::GadgetConfiguration`",
                ))
            }
            Fields::Unit => Err(Error::new_spanned(
                input_ident,
                "Context Extensions traits cannot be derived for unit structs",
            )),
        },
        _ => Err(Error::new_spanned(
            input_ident,
            "Context Extensions traits can only be derived for structs",
        )),
    }
}

fn generate_keystore_context_impl(
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
        impl #impl_generics_without_rwlock gadget_sdk::ctx::KeystoreContext<RwLock> for #name #ty_generics #where_clause_without_rwlock {
            fn keystore(&self) -> Result<gadget_sdk::keystore::backend::GenericKeyStore<RwLock>, gadget_sdk::config::Error> {
                #field_access.keystore()
            }
        }

        #[cfg(feature = "std")]
        impl #impl_generics gadget_sdk::ctx::KeystoreContext<gadget_sdk::ext::parking_lot::RawRwLock> for #name #ty_generics #where_clause {
            fn keystore(&self) -> Result<gadget_sdk::keystore::backend::GenericKeyStore<gadget_sdk::ext::parking_lot::RawRwLock>, gadget_sdk::config::Error> {
                #field_access.keystore()
            }
        }
    }
}
