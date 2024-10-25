use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_additional_tangle_logic(struct_name: &Ident) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl gadget_sdk::event_listener::markers::IsTangle for #struct_name {}
    }
}
