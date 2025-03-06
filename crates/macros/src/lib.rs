//! Macros for [`blueprint_sdk`].
//!
//! [`blueprint_sdk`]: https://crates.io/crates/blueprint_sdk
#![doc(
    html_logo_url = "https://cdn.prod.website-files.com/6494562b44a28080aafcbad4/65aaf8b0818b1d504cbdf81b_Tnt%20Logo.png"
)]
#![cfg_attr(test, allow(clippy::float_cmp))]
#![cfg_attr(not(test), warn(clippy::print_stdout, clippy::dbg_macro))]

use debug_job::FunctionKind;
use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{Type, parse::Parse};

mod attr_parsing;
mod debug_job;
mod from_ref;
mod with_position;

/// Generates better error messages when applied to job functions.
///
/// While using [`blueprint_sdk`], you can get long error messages for simple mistakes. For example:
///
/// ```compile_fail
/// use blueprint_sdk::Router;
///
/// #[tokio::main]
/// async fn main() {
///   Router::<()>::new().route(0, job);
/// }
///
/// fn job() -> &'static str {
///     "Hello, world"
/// }
/// ```
///
/// You will get a long error message about function not implementing [`Job`] trait. But why
/// does this function not implement it? To figure it out, the [`debug_job`] macro can be used.
///
/// ```compile_fail
/// # use blueprint_sdk::Router;
/// # use blueprint_macros::debug_job;
/// #
/// # #[tokio::main]
/// # async fn main() {
/// #   Router::new().route(0, job);
/// # }
/// #
/// #[debug_job]
/// fn job() -> &'static str {
///     "Hello, world"
/// }
/// ```
///
/// ```text
/// error: jobs must be async functions
///   --> main.rs:xx:1
///    |
/// xx | fn job() -> &'static str {
///    | ^^
/// ```
///
/// As the error message says, job function needs to be async.
///
/// ```no_run
/// use blueprint_sdk::{Router, debug_job};
///
/// #[tokio::main]
/// async fn main() {
///     Router::<()>::new().route(0, job);
/// }
///
/// #[debug_job]
/// async fn job() -> &'static str {
///     "Hello, world"
/// }
/// ```
///
/// # Changing context type
///
/// By default `#[debug_job]` assumes your context type is `()` unless your job has a
/// [`blueprint_sdk::Context`] argument:
///
/// ```
/// use blueprint_sdk::debug_job;
/// use blueprint_sdk::extract::Context;
///
/// #[debug_job]
/// async fn job(
///     // this makes `#[debug_job]` use `AppContext`
///     Context(context): Context<AppContext>,
/// ) {
/// }
///
/// #[derive(Clone)]
/// struct AppContext {}
/// ```
///
/// If your job takes multiple [`blueprint_sdk::Context`] arguments or you need to otherwise
/// customize the context type you can set it with `#[debug_job(context = ...)]`:
///
/// ```
/// use blueprint_sdk::extract::Context;
/// use blueprint_sdk::{debug_job, extract::FromRef};
///
/// #[debug_job(context = AppContext)]
/// async fn job(Context(app_ctx): Context<AppContext>, Context(inner_ctx): Context<InnerContext>) {
/// }
///
/// #[derive(Clone)]
/// struct AppContext {
///     inner: InnerContext,
/// }
///
/// #[derive(Clone)]
/// struct InnerContext {}
///
/// impl FromRef<AppContext> for InnerContext {
///     fn from_ref(context: &AppContext) -> Self {
///         context.inner.clone()
///     }
/// }
/// ```
///
/// # Limitations
///
/// This macro does not work for functions in an `impl` block that don't have a `self` parameter:
///
/// ```compile_fail
/// use blueprint_sdk::debug_job;
/// use blueprint_tangle_extra::extract::TangleArg;
///
/// struct App {}
///
/// impl App {
///     #[debug_job]
///     async fn my_job(TangleArg(_): TangleArg<u64>) {}
/// }
/// ```
///
/// This will yield an error similar to this:
///
/// ```text
/// error[E0425]: cannot find function `__blueprint_macros_check_job_0_from_job_call_check` in this scope
//    --> src/main.rs:xx:xx
//     |
//  xx |     pub async fn my_job(TangleArg(_): TangleArg<u64>)  {}
//     |                                   ^^^^ not found in this scope
/// ```
/// 
/// # Performance
///
/// This macro has no effect when compiled with the release profile. (eg. `cargo build --release`)
///
/// [`blueprint_sdk`]: https://docs.rs/blueprint_sdk/0.1
/// [`Job`]: https://docs.rs/blueprint_sdk/0.1/blueprint_sdk/job/trait.Job.html
/// [`blueprint_sdk::Context`]: https://docs.rs/blueprint_sdk/0.1/blueprint_sdk/context/struct.Context.html
/// [`debug_job`]: macro@debug_job
#[proc_macro_attribute]
pub fn debug_job(_attr: TokenStream, input: TokenStream) -> TokenStream {
    #[cfg(not(debug_assertions))]
    return input;

    #[cfg(debug_assertions)]
    #[allow(clippy::used_underscore_binding)]
    return expand_attr_with(_attr, input, |attrs, item_fn| {
        debug_job::expand(attrs, &item_fn, FunctionKind::Job)
    });
}

/// Derive an implementation of [`FromRef`] for each field in a struct.
///
/// # Example
///
/// ```
/// use blueprint_sdk::{
///     Router,
///     extract::{Context, FromRef},
/// };
///
/// #
/// # type Keystore = String;
/// # type DatabasePool = ();
/// #
/// // This will implement `FromRef` for each field in the struct.
/// #[derive(FromRef, Clone)]
/// struct AppContext {
///     keystore: Keystore,
///     database_pool: DatabasePool,
///     // fields can also be skipped
///     #[from_ref(skip)]
///     openai_api_token: String,
/// }
///
/// // So those types can be extracted via `Context`
/// async fn my_job(Context(keystore): Context<Keystore>) {}
///
/// async fn other_job(Context(database_pool): Context<DatabasePool>) {}
///
/// # let keystore = Default::default();
/// # let database_pool = Default::default();
/// let ctx = AppContext {
///     keystore,
///     database_pool,
///     openai_api_token: "sk-secret".to_owned(),
/// };
///
/// let router = Router::new()
///     .route(0, my_job)
///     .route(1, other_job)
///     .with_context(ctx);
/// # let _: Router = router;
/// ```
///
/// [`FromRef`]: https://docs.rs/blueprint-sdk/0.1/blueprint_sdk/extract/trait.FromRef.html
#[proc_macro_derive(FromRef, attributes(from_ref))]
pub fn derive_from_ref(item: TokenStream) -> TokenStream {
    expand_with(item, from_ref::expand)
}

fn expand_with<F, I, K>(input: TokenStream, f: F) -> TokenStream
where
    F: FnOnce(I) -> syn::Result<K>,
    I: Parse,
    K: ToTokens,
{
    expand(syn::parse(input).and_then(f))
}

fn expand_attr_with<F, A, I, K>(attr: TokenStream, input: TokenStream, f: F) -> TokenStream
where
    F: FnOnce(A, I) -> K,
    A: Parse,
    I: Parse,
    K: ToTokens,
{
    let expand_result = (|| {
        let attr = syn::parse(attr)?;
        let input = syn::parse(input)?;
        Ok(f(attr, input))
    })();
    expand(expand_result)
}

fn expand<T>(result: syn::Result<T>) -> TokenStream
where
    T: ToTokens,
{
    match result {
        Ok(tokens) => {
            let tokens = (quote! { #tokens }).into();
            if std::env::var_os("BLUEPRINT_MACROS_DEBUG").is_some() {
                eprintln!("{tokens}");
            }
            tokens
        }
        Err(err) => err.into_compile_error().into(),
    }
}

fn infer_context_types<'a, I>(types: I) -> impl Iterator<Item = Type> + 'a
where
    I: Iterator<Item = &'a Type> + 'a,
{
    types
        .filter_map(|ty| {
            if let Type::Path(path) = ty {
                Some(&path.path)
            } else {
                None
            }
        })
        .filter_map(|path| {
            if let Some(last_segment) = path.segments.last() {
                if last_segment.ident != "Context" {
                    return None;
                }

                match &last_segment.arguments {
                    syn::PathArguments::AngleBracketed(args) if args.args.len() == 1 => {
                        Some(args.args.first().unwrap())
                    }
                    _ => None,
                }
            } else {
                None
            }
        })
        .filter_map(|generic_arg| {
            if let syn::GenericArgument::Type(ty) = generic_arg {
                Some(ty)
            } else {
                None
            }
        })
        .cloned()
}

#[cfg(test)]
fn run_ui_tests(directory: &str) {
    #[rustversion::nightly]
    fn go(directory: &str) {
        let t = trybuild::TestCases::new();

        if let Ok(mut path) = std::env::var("BLUEPRINT_TEST_ONLY") {
            if let Some(path_without_prefix) = path.strip_prefix("macros/") {
                path = path_without_prefix.to_owned();
            }

            if !path.contains(&format!("/{directory}/")) {
                return;
            }

            if path.contains("/fail/") {
                t.compile_fail(path);
            } else if path.contains("/pass/") {
                t.pass(path);
            } else {
                panic!()
            }
        } else {
            t.compile_fail(format!("tests/{directory}/fail/*.rs"));
            t.pass(format!("tests/{directory}/pass/*.rs"));
        }
    }

    #[rustversion::not(nightly)]
    fn go(_directory: &str) {}

    go(directory);
}
