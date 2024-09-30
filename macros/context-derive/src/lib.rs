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

/// Field information for the configuration field.
mod cfg;
/// EVM Provider context extension implementation.
mod evm;
/// Keystore context extension implementation.
mod keystore;
/// Services context extension implementation.
mod services;
/// Tangle Subxt Client context extension implementation.
mod subxt;

/// Derive macro for generating Context Extensions trait implementation for `KeystoreContext`.
#[proc_macro_derive(KeystoreContext, attributes(config))]
pub fn derive_keystore_context(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let result = cfg::find_config_field(&input.ident, &input.data)
        .map(|config_field| keystore::generate_context_impl(input, config_field));

    match result {
        Ok(expanded) => TokenStream::from(expanded),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

/// Derive macro for generating Context Extensions trait implementation for `EVMProviderContext`.
#[proc_macro_derive(EVMProviderContext, attributes(config))]
pub fn derive_evm_provider_context(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let result = cfg::find_config_field(&input.ident, &input.data)
        .map(|config_field| evm::generate_context_impl(input, config_field));

    match result {
        Ok(expanded) => TokenStream::from(expanded),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

/// Derive macro for generating Context Extensions trait implementation for `TangleClientContext`.
#[proc_macro_derive(TangleClientContext, attributes(config))]
pub fn derive_tangle_client_context(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let result = cfg::find_config_field(&input.ident, &input.data)
        .map(|config_field| subxt::generate_context_impl(input, config_field));

    match result {
        Ok(expanded) => TokenStream::from(expanded),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

/// Derive macro for generating Context Extensions trait implementation for `ServicesContext`.
#[proc_macro_derive(ServicesContext, attributes(config))]
pub fn derive_services_context(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let result = cfg::find_config_field(&input.ident, &input.data)
        .map(|config_field| services::generate_context_impl(input, config_field));

    match result {
        Ok(expanded) => TokenStream::from(expanded),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}
