use proc_macro2::{Ident, TokenStream};
use quote::quote_spanned;
use syn::{
    Field, ItemStruct, Token, Type,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

use crate::attr_parsing::{Combine, combine_unary_attribute, parse_attrs};

pub(crate) fn expand(item: ItemStruct) -> syn::Result<TokenStream> {
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            item.generics,
            "`#[derive(FromRef)]` doesn't support generics",
        ));
    }

    let tokens = item
        .fields
        .iter()
        .enumerate()
        .map(|(idx, field)| expand_field(&item.ident, idx, field))
        .collect();

    Ok(tokens)
}

fn expand_field(context: &Ident, idx: usize, field: &Field) -> TokenStream {
    let FieldAttrs { skip } = match parse_attrs("from_ref", &field.attrs) {
        Ok(attrs) => attrs,
        Err(err) => return err.into_compile_error(),
    };

    if skip.is_some() {
        return TokenStream::default();
    }

    let field_ty = &field.ty;
    let span = field.ty.span();

    let body = if let Some(field_ident) = &field.ident {
        if matches!(field_ty, Type::Reference(_)) {
            quote_spanned! {span=> context.#field_ident }
        } else {
            quote_spanned! {span=> context.#field_ident.clone() }
        }
    } else {
        let idx = syn::Index {
            index: u32::try_from(idx)
                .expect("number of fields fits in u32 otherwise we have bigger problems"),
            span: field.span(),
        };
        quote_spanned! {span=> context.#idx.clone() }
    };

    quote_spanned! {span=>
        #[allow(clippy::clone_on_copy, clippy::clone_on_ref_ptr)]
        impl ::blueprint_sdk::extract::FromRef<#context> for #field_ty {
            fn from_ref(context: &#context) -> Self {
                #body
            }
        }
    }
}

mod kw {
    syn::custom_keyword!(skip);
}

#[derive(Default)]
pub(super) struct FieldAttrs {
    pub(super) skip: Option<kw::skip>,
}

impl Parse for FieldAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut skip = None;

        while !input.is_empty() {
            let lh = input.lookahead1();
            if lh.peek(kw::skip) {
                skip = Some(input.parse()?);
            } else {
                return Err(lh.error());
            }

            let _ = input.parse::<Token![,]>();
        }

        Ok(Self { skip })
    }
}

impl Combine for FieldAttrs {
    fn combine(mut self, other: Self) -> syn::Result<Self> {
        let Self { skip } = other;
        combine_unary_attribute(&mut self.skip, skip)?;
        Ok(self)
    }
}

#[test]
fn ui() {
    crate::run_ui_tests("from_ref");
}
