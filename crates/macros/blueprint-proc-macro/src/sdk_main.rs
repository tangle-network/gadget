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
    // Presented as #[sdk::main(skip_logger)]
    syn::custom_keyword!(skip_logger);
}

// Add fields which are passed onto tokio, and those that are reserved for the sdk_main macro
pub(crate) struct SdkMainArgs {
    tokio_args: Option<Vec<Expr>>,
    env: bool,
    skip_logger: bool,
}

pub(crate) fn sdk_main_impl(args: &SdkMainArgs, input: &ItemFn) -> syn::Result<TokenStream> {
    #[cfg(not(feature = "std"))]
    if args.env {
        use syn::spanned::Spanned;

        return Err(syn::Error::new(
            input.span(),
            "`env` creation requires the `std` feature",
        ));
    }

    let tokio_args = if let Some(args) = &args.tokio_args {
        quote! { ( crate = "gadget_macros::ext::tokio", #(#args),* ) }
    } else {
        quote! { ( crate = "gadget_macros::ext::tokio" ) }
    };

    let env_function_signature = if args.env {
        quote! { env: gadget_macros::ext::config::GadgetConfiguration }
    } else {
        quote! {}
    };

    let env_passed_var = if args.env {
        quote! { env }
    } else {
        quote! {}
    };

    let standard_setup = if args.env {
        quote! {
            // Load the environment and create the gadget runner
            let config: gadget_macros::ext::config::ContextConfig = gadget_macros::ext::clap::Parser::parse();
            let env = gadget_macros::ext::config::load(config.clone()).expect("Failed to load environment");
        }
    } else {
        proc_macro2::TokenStream::default()
    };

    let logger = if args.skip_logger {
        quote! {}
    } else {
        quote! {
            gadget_macros::ext::logging::setup_log();
        }
    };

    // Next, we need to consider the input. It should be in the form "async fn main() { ... }"
    // We must remove all the async fn main() and keep everything else
    let main_ret = match &input.sig.output {
        syn::ReturnType::Default => quote! { Result<(), Box<dyn core::error::Error>> },
        syn::ReturnType::Type(_, ty) => quote! { #ty },
    };
    let input_attrs = input.attrs.iter();
    let input = input.block.clone();

    let tokens = quote! {
        #[gadget_macros::ext::tokio::main #tokio_args]
        async fn main() -> Result<(), Box<dyn core::error::Error>> {
            #logger
            #standard_setup
            inner_main(#env_passed_var).await?;
            Ok(())
        }

        #(#input_attrs)*
        async fn inner_main(#env_function_signature) -> #main_ret {
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
        let mut skip_logger = false;
        // Parse through everything
        while !input.is_empty() {
            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>()?;
            }

            if input.peek(kw::env) {
                let _ = input.parse::<kw::env>()?;
                env = true;
            } else if input.peek(kw::skip_logger) {
                let _ = input.parse::<kw::skip_logger>()?;
                skip_logger = true;
            } else {
                // Parse the input as an expression
                tokio_args.push(input.parse()?);
            }
        }

        let tokio_args = if tokio_args.is_empty() {
            None
        } else {
            Some(tokio_args)
        };

        Ok(SdkMainArgs {
            tokio_args,
            env,
            skip_logger,
        })
    }
}
