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
    let field_access = match config_field {
        FieldInfo::Named(ident) => quote! { self.#ident },
        FieldInfo::Unnamed(index) => quote! { self.#index },
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        #[::gadget_macros::ext::async_trait::async_trait]
        impl #impl_generics ::gadget_macros::ext::contexts::p2p::P2pContext for #name #ty_generics #where_clause {
            fn p2p_client(
                &self,
                name: ::gadget_macros::ext::std::string::String,
                target_port: u16,
                my_ecdsa_key: gadget_macros::ext::contexts::p2p::GossipMsgKeyPair,
            ) -> ::gadget_macros::ext::contexts::p2p::P2PClient {
                ::gadget_macros::ext::contexts::p2p::P2PClient::new(
                    name,
                    #field_access.clone(),
                    target_port,
                    my_ecdsa_key.clone()
                )
            }
        }
    }
}
