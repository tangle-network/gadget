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

mod blueprint;
/// Blueprint Hooks proc-macro
mod hooks;
/// Blueprint Job proc-macro
mod job;
/// Quality of service functions for misbehavior reports
mod qos;
/// Report proc-macro
mod report;

/// Utilities for the Blueprint Macros
mod utils;

/// A procedural macro that annotates a function as a job.
///
/// # Example
/// ```rust,no_run
/// # use gadget_blueprint_proc_macro::job;
///
/// /// A simple add function that adds two numbers.
/// #[job(id = 0, params(a, b), result(u32))]
/// pub fn add(a: u32, b: u32) -> u32 {
///    a + b
/// }
/// ```
///
/// The `job` macro generates the following code:
/// ```rust
/// /// A Job Definition ID for the [`add`] function.
/// #[automatically_derived]
/// pub const ADD_JOB_ID: u8 = 0;
///
/// /// A Job Definition for the [`add`] function.
/// #[automatically_derived]
/// pub const ADD_JOB_DEF: &str = r#"{"metadata":{"name":"add","description":"A simple add function that adds two numbers"},"params":["Uint32", "Uint32"],"result":["Uint32"],"verifier":"None"}"#;
///
/// /// A simple add function that adds two numbers.
/// pub fn add(a: u32, b: u32) -> u32 {
///   a + b
/// }
///
/// pub struct AddEventHandler {
///    pub service_id: u64,
///    pub signer: subxt_signer::sr25519::Keypair,
///    // ... other fields
/// }
/// ```
/// Addon to the generated code, the `job` macro also generates an Event Handler struct that
/// implements the `EventHandler` trait for you.
///
/// # Parameters
/// - `id`: The unique identifier for the job (must be in the range of 0..[`u8::MAX`])
/// - `params`: The parameters of the job function, must be a tuple of identifiers in the function signature.
/// - `result`: The result of the job function, must be a type that this job returns.
///    also, it can be omitted if the return type is simple to infer, like `u32` or `Vec<u8>` just use `_`.
/// - `skip_codegen`: A flag to skip the code generation for the job, useful for manual event
/// handling.
#[proc_macro_attribute]
pub fn job(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as job::JobArgs);
    let input = parse_macro_input!(input as syn::ItemFn);

    match job::job_impl(&args, &input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

///
/// The `report` macro is used to annotate a function as a report handler for misbehaviors. This macro generates
/// the necessary code to handle events and process reports within the service blueprint. Reports are specifically
/// for submitting incorrect job results, attributable malicious behavior, or otherwise machine failures and reliability degradation.
///
/// # Example
/// ```rust
/// # use gadget_blueprint_proc_macro::report;
/// #[report(id = 1, params(a, b, c), result(u32))]
/// pub fn report_add(a: u32, b: u32, c: SignedResult<u32>) -> u32 {
///    if a + b == c {
///       0
///    } else {
///      1
///    }
/// }
/// ```
///
/// The `report` macro generates the following code:
/// ```rust
/// /// A Report Definition ID for the [`report_add`] function.
/// #[automatically_derived]
/// pub const REPORT_ADD_REPORT_ID: u8 = 1;
///
/// /// A Report Definition for the [`report_add`] function.
/// #[automatically_derived]
/// pub const REPORT_ADD_REPORT_DEF: &str = r#"{"metadata":{"name":"report_add","description":"A function to report the incorrect addition of two numbers"},"params":["Uint32", "Uint32", "Uint32"],"result":["Uint32"],"verifier":"None"}"#;
///
/// /// A function to report the addition of two numbers.
/// pub fn report_add(a: u32, b: u32, c: SignedResult<u32>) -> u32 {
///    if a + b == c {
///       0
///    } else {
///      1
///    }
/// }
///
/// pub struct ReportAddEventHandler {
///    pub service_id: u64,
///    pub signer: subxt_signer::sr25519::Keypair,
///    // ... other fields
/// }
/// ```
/// In addition to the generated code, the `report` macro also generates an Event Handler struct that
/// implements the `EventHandler` trait for you.
///
/// # Parameters
/// - `id`: The unique identifier for the report (must be in the range of 0..[`u8::MAX`])
/// - `params`: The parameters of the report function, must be a tuple of identifiers in the function signature.
/// - `result`: The result of the report function, must be a type that this report returns.
///    It can be omitted if the return type is simple to infer, like `u32` or `Vec<u8>` by using `_`.
/// - `skip_codegen`: A flag to skip the code generation for the report, useful for manual event
/// handling.
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
/// # Example
/// ```rust,no_run
/// # use gadget_blueprint_proc_macro::registration_hook;
/// #[registration_hook(evm = "MyRegistrationHook")]
/// pub fn my_registration_hook();
/// ```
#[proc_macro_attribute]
pub fn registration_hook(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ForeignItemFn);
    let args = parse_macro_input!(args as hooks::HookArgs);
    match hooks::registration_hook_impl(&args, &input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

/// A procedural macro that annotates a function as a request hook, mainly used
/// for the metadata in the `blueprint.json`.
/// # Example
/// ```rust,no_run
/// # use gadget_blueprint_proc_macro::request_hook;
/// #[request_hook(evm = "MyRequestHook")]
/// pub fn my_request_hook();
/// ```
#[proc_macro_attribute]
pub fn request_hook(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ForeignItemFn);
    let args = parse_macro_input!(args as hooks::HookArgs);
    match hooks::request_hook_impl(&args, &input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}
