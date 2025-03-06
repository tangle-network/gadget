use std::path::PathBuf;
use proc_macro::TokenStream;
use quote::quote;
use serde_json::Value;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Ident, LitStr, Token};

struct LoadAbiArgs {
    ident: Ident,
    _comma: Token![,],
    file_path: LitStr,
}

impl Parse for LoadAbiArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(LoadAbiArgs {
            ident: input.parse()?,
            _comma: input.parse()?,
            file_path: input.parse()?,
        })
    }
}

pub fn load_abi(input: TokenStream) -> TokenStream {
    let LoadAbiArgs {
        ident, file_path, ..
    } = parse_macro_input!(input as LoadAbiArgs);
    let file_path = file_path.value();

    let crate_root = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let absolute_path = PathBuf::from(crate_root).join(file_path.clone());

    if !absolute_path.exists() {
        return syn::Error::new_spanned(
            file_path,
            format!("ABI file not found at: {}", absolute_path.display()),
        )
            .to_compile_error()
            .into();
    }

    let file_content = std::fs::read_to_string(&absolute_path).expect("Failed to read ABI file");

    let json: Value = serde_json::from_str(&file_content).expect("Failed to parse JSON");

    let abi = json["abi"].to_string();

    quote! {
        const #ident: &str = #abi;
    }
        .into()
}
