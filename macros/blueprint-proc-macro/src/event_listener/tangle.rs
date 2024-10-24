use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, LitInt};

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_tangle_event_handler(
    struct_name: &Ident,
    _job_id: &LitInt,
    _params_tokens: &[TokenStream],
    _result_tokens: &[TokenStream],
    _fn_call: &TokenStream,
) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl gadget_sdk::event_listener::markers::IsTangle for #struct_name {}
    }
}
