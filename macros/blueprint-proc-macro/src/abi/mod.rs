use proc_macro::TokenStream;
use quote::quote;
use std::path::{Path, PathBuf};
use syn::{parse_macro_input, LitStr};

pub fn load_abi(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);
    let file_path = input.value();

    let workspace_dir = workspace_dir();
    let absolute_path = workspace_dir.join(file_path);

    if !absolute_path.exists() {
        return syn::Error::new_spanned(
            input,
            format!("ABI file not found at: {}", absolute_path.display()),
        )
        .to_compile_error()
        .into();
    }

    let absolute_path_str = absolute_path.to_str().unwrap();

    quote! {
        {
            let abi_content = std::fs::read_to_string(#absolute_path_str)
                .expect("Failed to read ABI file");
            serde_json::from_str::<alloy_json_abi::JsonAbi>(&abi_content)
                .expect("Failed to parse ABI JSON")
        }
    }
    .into()
}

fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}
