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
//! Blueprint Macros

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Abi proc-macro
#[cfg(feature = "std")]
mod abi;
/// Benchmarking proc-macro
mod benchmark;
/// Blueprint Hooks proc-macro
mod hooks;
/// Blueprint Job proc-macro
mod job;
/// Report proc-macro
mod report;
/// Shared utilities for the Blueprint Macros
mod shared;

mod sdk_main;

/// A procedural macro that annotates a function as a job.
///
/// Addon to the generated code, the `job` macro also generates an Event Handler struct that
/// implements the `EventHandler` trait for you.
///
/// # Parameters
/// - `id`: The unique identifier for the job (must be in the range of 0..[`u8::MAX`])
/// - `params`: The parameters of the job function, must be a tuple of identifiers in the function signature.
/// - `result`: The result of the job function, must be a type that this job returns.
///    also, it can be omitted if the return type is simple to infer, like `u32` or `Vec<u8>` just use `_`.
/// - `skip_codegen`: A flag to skip the code generation for the job, useful for manual event handling.
#[proc_macro_attribute]
pub fn job(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as job::JobArgs);
    let input = parse_macro_input!(input as syn::ItemFn);

    match job::job_impl(args, input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Creates a misbehavior report handler for the given function.
///
/// This macro generates the necessary code to handle events and process reports within the
/// service blueprint. Reports are specifically for submitting incorrect job results, attributable
/// malicious behavior, or otherwise machine failures and reliability degradation.
///
/// In addition to the generated code, the `report` macro also generates an Event Handler struct that
/// implements the `EventHandler` trait for you.
///
/// # Parameters
/// - `id`: The unique identifier for the report (must be in the range of 0..[`u8::MAX`])
/// - `params`: The parameters of the report function, must be a tuple of identifiers in the function signature.
/// - `result`: The result of the report function, must be a type that this report returns.
///    It can be omitted if the return type is simple to infer, like `u32` or `Vec<u8>` by using `_`.
/// - `skip_codegen`: A flag to skip the code generation for the report, useful for manual event handling.
#[proc_macro_attribute]
pub fn report(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as report::ReportArgs);
    let input = parse_macro_input!(input as syn::ItemFn);

    match report::report_impl(&args, &input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// A procedural macro that annotates a function as a registration hook, mainly used
/// for the metadata in the `blueprint.json`.
#[proc_macro_attribute]
pub fn registration_hook(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ForeignItemFn);
    match hooks::registration_hook_impl(&input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// A procedural macro that annotates a function as a request hook, mainly used
/// for the metadata in the `blueprint.json`.
#[proc_macro_attribute]
pub fn request_hook(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ForeignItemFn);
    match hooks::request_hook_impl(&input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// A procedural macro that annotates a function as a benchmark hook, mainly used
/// during the benchmarking phase.
#[proc_macro_attribute]
pub fn benchmark(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemFn);
    let args = parse_macro_input!(args as benchmark::BenchmarkArgs);
    match benchmark::benchmark_impl(&args, &input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// A procedural macro that outputs the JsonAbi for the given file path.
#[proc_macro]
#[cfg(feature = "std")]
pub fn load_abi(input: TokenStream) -> TokenStream {
    abi::load_abi(input)
}

/// A procedural macro that annotates a function as a main function for the blueprint.
///
/// ```ignore
/// #[blueprint_sdk::main(env)]
/// pub async fn main() {
///    // Your main function code here
/// }
/// ```
///
/// # Parameters
/// - `env`: Sets up the environment for the main function.
/// - `skip_logging`: A flag to skip the logging setup for the main function.
/// - ...: are passes as-is to the `#[tokio::main]` attribute.
#[proc_macro_attribute]
pub fn main(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemFn);
    let args = parse_macro_input!(args as sdk_main::SdkMainArgs);
    match sdk_main::sdk_main_impl(&args, &input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}
