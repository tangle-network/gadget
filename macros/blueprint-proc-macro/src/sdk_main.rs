use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, ItemFn, Token};

// This macro contains the following format:
// #[sdk_main]
// or
// #[sdk_main(args...)]
// all args are passed into the tokio::main attribute, with exception of certain keywords
// all exception keywords are listed in mod kw below

mod kw {
    // Presented as #[sdk::main(env)]
    syn::custom_keyword!(env);
}

// Add fields which are passed onto tokio, and those that are reserved for the sdk_main macro
pub(crate) struct SdkMainArgs {
    tokio_args: Option<Vec<Expr>>,
    env: bool,
}

pub(crate) fn sdk_main_impl(args: &SdkMainArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    let tokio_args = if let Some(args) = &args.tokio_args {
        quote! { ( #(#args),* ) }
    } else {
        quote! {}
    };

    let env_function_signature = if args.env {
        quote! { env: gadget_sdk::config::GadgetConfiguration<gadget_sdk::parking_lot::RawRwLock> }
    } else {
        quote! {}
    };

    let env_passed_var = if args.env {
        quote! { env }
    } else {
        quote! {}
    };

    // Next, we need to consider the input. It should be in the form "async fn main() { ... }"
    // We must remove all the async fn main() and keep everything else
    let input = input.block.clone();

    let tokens = quote! {
        use gadget_sdk::tokio;
        #[tokio::main #tokio_args]
        async fn main() -> Result<(), Box<dyn std::error::Error>> {
            gadget_sdk::logging::setup_log();
            // Load the environment and create the gadget runner
            let config: gadget_sdk::config::ContextConfig = gadget_sdk::structopt::StructOpt::from_args();
            let env = gadget_sdk::config::load(config.clone()).expect("Failed to load environment");
            gadget_sdk::utils::check_for_test(&config).expect("Failed to check for test");

            inner_main(#env_passed_var).await?;
            Ok(())
        }

        async fn inner_main(#env_function_signature) -> Result<(), Box<dyn std::error::Error>> {
            #input
        }
    };

    Ok(tokens.into())
}

impl Parse for SdkMainArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // If any of the input contains one of the keywords in the kw module, add its corresponding field
        // to the SdkMainArgs struct. Otherwise, add it to the tokio args
        let mut tokio_args = vec![];
        let mut env = false;
        // Parse through everything
        while !input.is_empty() {
            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            }

            if input.peek(kw::env) {
                let _ = input.parse::<kw::env>()?;
                env = true;
            } else {
                tokio_args.push(input.parse()?);
            }
        }

        let tokio_args = if tokio_args.is_empty() {
            None
        } else {
            Some(tokio_args)
        };

        Ok(SdkMainArgs { tokio_args, env })
    }
}
