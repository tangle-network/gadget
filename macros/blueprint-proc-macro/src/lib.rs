#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    missing_docs,
    rustdoc::broken_intra_doc_links,
    unused_results,
    clippy::all,
    clippy::pedantic,
    clippy::exhaustive_enums
)]
//! Blueprint Macros

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Blueprint Job proc-macro
mod job;
mod blueprint;

/// A procedural macro that annotates a function as a job.
/// It generates a struct with the same name as the function (in `PascalCase`)
/// and with a postfix of `Job` that holds the function's arguments (passed in `params`)
/// and the function result (passed in `result`).
///
/// # Example
/// ```rust,ignore
/// # use blueprint_macro::job;
/// #[job(params(n, t), result(Bytes))]
/// fn keygen(n: u16, t: u8) -> Bytes {
/// // ...
/// }
/// ```
/// This will generate the following struct:
/// ```rust,ignore
/// #[derive(Debug, Serialize, Deserialize)]
/// struct KeygenJob {
///     params: (u16, u8),
///     result: Bytes
/// }
/// ```
#[proc_macro_attribute]
pub fn job(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as job::JobArgs);
    let input = parse_macro_input!(input as syn::ItemFn);

    match job::job_impl(&args, &input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// A procedural macro that generates a blueprint.json file
/// that contains the blueprint of the given module.
///
/// # Example
/// ```rust,ignore
/// # use blueprint_macro::blueprint;
/// blueprint! {
///  registration_hook: None,
///  registration_params: [],
/// }
#[proc_macro]
pub fn blueprint(input: TokenStream) -> TokenStream {
    let input_str = format!("ServiceBlueprint({input})");
    let ron = ron::Options::default().with_default_extension(ron::extensions::Extensions::all());
    let maybe_blueprint = ron.from_str(&input_str);
    let blueprint = match maybe_blueprint {
        Ok(blueprint) => blueprint,
        Err(err) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("Failed to create blueprint: {err}"),
            )
            .to_compile_error()
            .into()
        }
    };
    let ServiceBlueprint { request_hook, .. } = blueprint;
    let out = quote::quote! {
        /// Gadget Blueprint
        /// AUTO GENERATED MODULE.
        pub mod blueprint {
            pub static BLUEPRINT: ServiceBlueprint = ServiceBlueprint {
                request_hook: #request_hook,
                ..Default::default()
            };
        }
    };
    out.into()
}
