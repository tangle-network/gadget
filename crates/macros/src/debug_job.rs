use std::{collections::HashSet, fmt};

use crate::{
    attr_parsing::{parse_assignment_attribute, second},
    with_position::{Position, WithPosition},
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{FnArg, ItemFn, ReturnType, Token, Type, parse::Parse, spanned::Spanned};

pub(crate) fn expand(attr: Attrs, item_fn: &ItemFn, kind: FunctionKind) -> TokenStream {
    let Attrs { context_ty } = attr;

    let mut context_ty = context_ty.map(second);

    let check_extractor_count = check_extractor_count(item_fn, kind);

    let check_output_tuples = check_output_tuples(item_fn);
    let check_output_impls_into_job_result = if check_output_tuples.is_empty() {
        check_output_impls_into_job_result(item_fn)
    } else {
        check_output_tuples
    };

    // If the function is generic, we can't reliably check its inputs or whether the future it
    // returns is `Send`. Skip those checks to avoid unhelpful additional compiler errors.
    let check_inputs_and_future_send = if item_fn.sig.generics.params.is_empty() {
        let mut err = None;

        if context_ty.is_none() {
            let context_types_from_args = context_types_from_args(item_fn);

            #[allow(clippy::comparison_chain)]
            if context_types_from_args.len() == 1 {
                context_ty = context_types_from_args.into_iter().next();
            } else if context_types_from_args.len() > 1 {
                err = Some(
                    syn::Error::new(
                        Span::call_site(),
                        format!(
                            "can't infer context type, please add set it explicitly, as in \
                            `#[debug_{kind}(context = MyContextType)]`"
                        ),
                    )
                    .into_compile_error(),
                );
            }
        }

        err.unwrap_or_else(|| {
            let context_ty = context_ty.unwrap_or_else(|| syn::parse_quote!(()));

            let check_future_send = check_future_send(item_fn, kind);

            if let Some(check_input_order) = check_input_order(item_fn, kind) {
                quote! {
                    #check_input_order
                    #check_future_send
                }
            } else {
                let check_inputs_impls_from_request =
                    check_inputs_impls_from_request(item_fn, &context_ty, kind);

                quote! {
                    #check_inputs_impls_from_request
                    #check_future_send
                }
            }
        })
    } else {
        syn::Error::new_spanned(
            &item_fn.sig.generics,
            format!("`#[blueprint_macros::debug_{kind}]` doesn't support generic functions"),
        )
        .into_compile_error()
    };

    let middleware_takes_next_as_last_arg =
        matches!(kind, FunctionKind::Middleware).then(|| next_is_last_input(item_fn));

    quote! {
        #item_fn
        #check_extractor_count
        #check_output_impls_into_job_result
        #check_inputs_and_future_send
        #middleware_takes_next_as_last_arg
    }
}

#[derive(Clone, Copy)]
pub(crate) enum FunctionKind {
    Job,
    Middleware,
}

impl fmt::Display for FunctionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionKind::Job => f.write_str("job"),
            FunctionKind::Middleware => f.write_str("middleware"),
        }
    }
}

impl FunctionKind {
    fn name_uppercase_plural(self) -> &'static str {
        match self {
            FunctionKind::Job => "Jobs",
            FunctionKind::Middleware => "Middleware",
        }
    }
}

mod kw {
    syn::custom_keyword!(body);
    syn::custom_keyword!(context);
}

pub(crate) struct Attrs {
    context_ty: Option<(kw::context, Type)>,
}

impl Parse for Attrs {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let mut context_ty = None;

        while !input.is_empty() {
            let lh = input.lookahead1();
            if lh.peek(kw::context) {
                parse_assignment_attribute(input, &mut context_ty)?;
            } else {
                return Err(lh.error());
            }

            let _ = input.parse::<Token![,]>();
        }

        Ok(Self { context_ty })
    }
}

fn check_extractor_count(item_fn: &ItemFn, kind: FunctionKind) -> Option<TokenStream> {
    let max_extractors = 16;
    let inputs = item_fn
        .sig
        .inputs
        .iter()
        .filter(|arg| skip_next_arg(arg, kind))
        .count();
    if inputs <= max_extractors {
        None
    } else {
        let error_message = format!(
            "{} cannot take more than {max_extractors} arguments. \
            Use `(a, b): (ExtractorA, ExtractorA)` to further nest extractors",
            kind.name_uppercase_plural(),
        );
        let error = syn::Error::new_spanned(&item_fn.sig.inputs, error_message).to_compile_error();
        Some(error)
    }
}

#[allow(dead_code)]
fn extractor_idents(
    item_fn: &ItemFn,
    kind: FunctionKind,
) -> impl Iterator<Item = (usize, &syn::FnArg, &syn::Ident)> {
    item_fn
        .sig
        .inputs
        .iter()
        .filter(move |arg| skip_next_arg(arg, kind))
        .enumerate()
        .filter_map(|(idx, fn_arg)| match fn_arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(pat_type) => {
                if let Type::Path(type_path) = &*pat_type.ty {
                    type_path
                        .path
                        .segments
                        .last()
                        .map(|segment| (idx, fn_arg, &segment.ident))
                } else {
                    None
                }
            }
        })
}

#[allow(dead_code)]
fn check_path_extractor(item_fn: &ItemFn, kind: FunctionKind) -> TokenStream {
    let path_extractors = extractor_idents(item_fn, kind)
        .filter(|(_, _, ident)| *ident == "Path")
        .collect::<Vec<_>>();

    if path_extractors.len() > 1 {
        path_extractors
            .into_iter()
            .map(|(_, arg, _)| {
                syn::Error::new_spanned(
                    arg,
                    "Multiple parameters must be extracted with a tuple \
                    `Path<(_, _)>` or a struct `Path<YourParams>`, not by applying \
                    multiple `Path<_>` extractors",
                )
                .to_compile_error()
            })
            .collect()
    } else {
        quote! {}
    }
}

fn is_self_pat_type(typed: &syn::PatType) -> bool {
    let ident = if let syn::Pat::Ident(ident) = &*typed.pat {
        &ident.ident
    } else {
        return false;
    };

    ident == "self"
}

fn check_inputs_impls_from_request(
    item_fn: &ItemFn,
    context_ty: &Type,
    kind: FunctionKind,
) -> TokenStream {
    let takes_self = item_fn.sig.inputs.first().is_some_and(|arg| match arg {
        FnArg::Receiver(_) => true,
        FnArg::Typed(typed) => is_self_pat_type(typed),
    });

    WithPosition::new(
        item_fn
            .sig
            .inputs
            .iter()
            .filter(|arg| skip_next_arg(arg, kind)),
    )
    .enumerate()
    .map(|(idx, arg)| {
        let must_impl_from_request_parts = match &arg {
            Position::First(_) | Position::Middle(_) => true,
            Position::Last(_) | Position::Only(_) => false,
        };

        let arg = arg.into_inner();

        let (span, ty) = match arg {
            FnArg::Receiver(receiver) => {
                if receiver.reference.is_some() {
                    return syn::Error::new_spanned(receiver, "Jobs must only take owned values")
                        .into_compile_error();
                }

                let span = receiver.span();
                (span, syn::parse_quote!(Self))
            }
            FnArg::Typed(typed) => {
                let ty = &typed.ty;
                let span = ty.span();

                if is_self_pat_type(typed) {
                    (span, syn::parse_quote!(Self))
                } else {
                    (span, ty.clone())
                }
            }
        };

        let consumes_job_call = job_call_consuming_type_name(&ty).is_some();

        let check_fn = format_ident!(
            "__blueprint_macros_check_{}_{}_from_job_check",
            item_fn.sig.ident,
            idx,
            span = span,
        );

        let call_check_fn = format_ident!(
            "__blueprint_macros_check_{}_{}_from_job_call_check",
            item_fn.sig.ident,
            idx,
            span = span,
        );

        let call_check_fn_body = if takes_self {
            quote_spanned! {span=>
                Self::#check_fn();
            }
        } else {
            quote_spanned! {span=>
                #check_fn();
            }
        };

        let check_fn_generics = if must_impl_from_request_parts || consumes_job_call {
            quote! {}
        } else {
            quote! { <M> }
        };

        let from_request_bound = if must_impl_from_request_parts {
            quote_spanned! {span=>
                #ty: ::blueprint_sdk::FromJobCallParts<#context_ty> + Send
            }
        } else if consumes_job_call {
            quote_spanned! {span=>
                #ty: ::blueprint_sdk::FromJobCall<#context_ty> + Send
            }
        } else {
            quote_spanned! {span=>
                #ty: ::blueprint_sdk::FromJobCall<#context_ty, M> + Send
            }
        };

        quote_spanned! {span=>
            #[allow(warnings)]
            #[doc(hidden)]
            fn #check_fn #check_fn_generics()
            where
                #from_request_bound,
            {}

            // we have to call the function to actually trigger a compile error
            // since the function is generic, just defining it is not enough
            #[allow(warnings)]
            #[doc(hidden)]
            fn #call_check_fn()
            {
                #call_check_fn_body
            }
        }
    })
    .collect::<TokenStream>()
}

fn check_output_tuples(item_fn: &ItemFn) -> TokenStream {
    let elems = match &item_fn.sig.output {
        ReturnType::Type(_, ty) => match &**ty {
            Type::Tuple(tuple) => &tuple.elems,
            _ => return quote! {},
        },
        ReturnType::Default => return quote! {},
    };

    let job_ident = &item_fn.sig.ident;

    match elems.len() {
        0 => quote! {},
        n if n > 17 => syn::Error::new_spanned(
            &item_fn.sig.output,
            "Cannot return tuples with more than 17 elements",
        )
        .to_compile_error(),
        _ => WithPosition::new(elems)
            .enumerate()
            .map(|(idx, arg)| match arg {
                Position::First(ty) => match extract_clean_typename(ty).as_deref() {
                    Some("StatusCode" | "Response") => quote! {},
                    Some("Parts") => check_is_job_result_parts(ty, job_ident, idx),
                    Some(_) | None => {
                        if let Some(tn) = well_known_last_job_result_type(ty) {
                            syn::Error::new_spanned(
                                ty,
                                format!(
                                    "`{tn}` must be the last element \
                                    in a response tuple"
                                ),
                            )
                            .to_compile_error()
                        } else {
                            check_into_job_result_parts(ty, job_ident, idx)
                        }
                    }
                },
                Position::Middle(ty) => {
                    if let Some(tn) = well_known_last_job_result_type(ty) {
                        syn::Error::new_spanned(
                            ty,
                            format!("`{tn}` must be the last element in a job result tuple"),
                        )
                        .to_compile_error()
                    } else {
                        check_into_job_result_parts(ty, job_ident, idx)
                    }
                }
                Position::Last(ty) | Position::Only(ty) => check_into_job_result(job_ident, ty),
            })
            .collect::<TokenStream>(),
    }
}

fn check_into_job_result(job: &Ident, ty: &Type) -> TokenStream {
    let (span, ty) = (ty.span(), ty.clone());

    let check_fn = format_ident!(
        "__blueprint_macros_check_{job}_into_job_result_check",
        span = span,
    );

    let call_check_fn = format_ident!(
        "__blueprint_macros_check_{job}_into_job_result_call_check",
        span = span,
    );

    let call_check_fn_body = quote_spanned! {span=>
        #check_fn();
    };

    let from_request_bound = quote_spanned! {span=>
        #ty: ::blueprint_sdk::IntoJobResult
    };
    quote_spanned! {span=>
        #[allow(warnings)]
        #[allow(unreachable_code)]
        #[doc(hidden)]
        fn #check_fn()
        where
            #from_request_bound,
        {}

        // we have to call the function to actually trigger a compile error
        // since the function is generic, just defining it is not enough
        #[allow(warnings)]
        #[allow(unreachable_code)]
        #[doc(hidden)]
        fn #call_check_fn() {
            #call_check_fn_body
        }
    }
}

fn check_is_job_result_parts(ty: &Type, ident: &Ident, index: usize) -> TokenStream {
    let (span, ty) = (ty.span(), ty.clone());

    let check_fn = format_ident!(
        "__blueprint_macros_check_{}_is_job_result_parts_{index}_check",
        ident,
        span = span,
    );

    quote_spanned! {span=>
        #[allow(warnings)]
        #[allow(unreachable_code)]
        #[doc(hidden)]
        fn #check_fn(parts: #ty) -> ::blueprint_sdk::job_result::Parts {
            parts
        }
    }
}

fn check_into_job_result_parts(ty: &Type, ident: &Ident, index: usize) -> TokenStream {
    let (span, ty) = (ty.span(), ty.clone());

    let check_fn = format_ident!(
        "__blueprint_macros_check_{}_into_job_result_parts_{index}_check",
        ident,
        span = span,
    );

    let call_check_fn = format_ident!(
        "__blueprint_macros_check_{}_into_job_result_parts_{index}_call_check",
        ident,
        span = span,
    );

    let call_check_fn_body = quote_spanned! {span=>
        #check_fn();
    };

    let from_job_call_bound = quote_spanned! {span=>
        #ty: ::blueprint_sdk::job_result::IntoJobResultParts
    };
    quote_spanned! {span=>
        #[allow(warnings)]
        #[allow(unreachable_code)]
        #[doc(hidden)]
        fn #check_fn()
        where
            #from_job_call_bound,
        {}

        // we have to call the function to actually trigger a compile error
        // since the function is generic, just defining it is not enough
        #[allow(warnings)]
        #[allow(unreachable_code)]
        #[doc(hidden)]
        fn #call_check_fn() {
            #call_check_fn_body
        }
    }
}

fn check_input_order(item_fn: &ItemFn, kind: FunctionKind) -> Option<TokenStream> {
    let number_of_inputs = item_fn
        .sig
        .inputs
        .iter()
        .filter(|arg| skip_next_arg(arg, kind))
        .count();

    let types_that_consume_the_request = item_fn
        .sig
        .inputs
        .iter()
        .filter(|arg| skip_next_arg(arg, kind))
        .enumerate()
        .filter_map(|(idx, arg)| {
            let ty = match arg {
                FnArg::Typed(pat_type) => &*pat_type.ty,
                FnArg::Receiver(_) => return None,
            };
            let type_name = job_call_consuming_type_name(ty)?;

            Some((idx, type_name, ty.span()))
        })
        .collect::<Vec<_>>();

    if types_that_consume_the_request.is_empty() {
        return None;
    }

    // exactly one type that consumes the request
    if types_that_consume_the_request.len() == 1 {
        // and that is not the last
        if types_that_consume_the_request[0].0 != number_of_inputs - 1 {
            let (_idx, type_name, span) = &types_that_consume_the_request[0];
            let error = format!(
                "`{type_name}` consumes the request body and thus must be \
                the last argument to the job function"
            );
            return Some(quote_spanned! {*span=>
                compile_error!(#error);
            });
        }
        return None;
    }

    if types_that_consume_the_request.len() == 2 {
        let (_, first, _) = &types_that_consume_the_request[0];
        let (_, second, _) = &types_that_consume_the_request[1];
        let error = format!(
            "Can't have two extractors that consume the request body. \
            `{first}` and `{second}` both do that.",
        );
        let span = item_fn.sig.inputs.span();
        Some(quote_spanned! {span=>
            compile_error!(#error);
        })
    } else {
        let types = WithPosition::new(types_that_consume_the_request)
            .map(|pos| match pos {
                Position::First((_, type_name, _)) | Position::Middle((_, type_name, _)) => {
                    format!("`{type_name}`, ")
                }
                Position::Last((_, type_name, _)) => format!("and `{type_name}`"),
                Position::Only(_) => unreachable!(),
            })
            .collect::<String>();

        let error = format!(
            "Can't have more than one extractor that consume the request body. \
            {types} all do that.",
        );
        let span = item_fn.sig.inputs.span();
        Some(quote_spanned! {span=>
            compile_error!(#error);
        })
    }
}

fn extract_clean_typename(ty: &Type) -> Option<String> {
    let path = match ty {
        Type::Path(type_path) => &type_path.path,
        _ => return None,
    };
    path.segments.last().map(|p| p.ident.to_string())
}

fn job_call_consuming_type_name(ty: &Type) -> Option<&'static str> {
    let typename = extract_clean_typename(ty)?;

    let typename = match &*typename {
        "TangleArg" => "TangleArg<_>",
        "TangleArgs2" => "TangleArgs2<_, _>",
        "TangleArgs3" => "TangleArgs3<_, _, _>",
        "TangleArgs4" => "TangleArgs4<_, _, _, _>",
        "TangleArgs5" => "TangleArgs5<_, _, _, _, _>",
        "TangleArgs6" => "TangleArgs6<_, _, _, _, _, _>",
        "TangleArgs7" => "TangleArgs7<_, _, _, _, _, _, _>",
        "TangleArgs8" => "TangleArgs8<_, _, _, _, _, _, _, _>",
        "TangleArgs9" => "TangleArgs9<_, _, _, _, _, _, _, _, _>",
        "TangleArgs10" => "TangleArgs10<_, _, _, _, _, _, _, _, _, _>",
        "TangleArgs11" => "TangleArgs11<_, _, _, _, _, _, _, _, _, _, _>",
        "TangleArgs12" => "TangleArgs12<_, _, _, _, _, _, _, _, _, _, _, _>",
        "TangleArgs13" => "TangleArgs13<_, _, _, _, _, _, _, _, _, _, _, _>",
        "TangleArgs14" => "TangleArgs14<_, _, _, _, _, _, _, _, _, _, _, _, _>",
        "TangleArgs15" => "TangleArgs15<_, _, _, _, _, _, _, _, _, _, _, _, _, _>",
        "TangleArgs16" => "TangleArgs16<_, _, _, _, _, _, _, _, _, _, _, _, _, _, _>",

        "JobCall" => "JobCall<_>",
        "Bytes" => "Bytes",
        "String" => "String",
        "Parts" => "Parts",
        _ => return None,
    };

    Some(typename)
}

fn well_known_last_job_result_type(ty: &Type) -> Option<&'static str> {
    let typename = extract_clean_typename(ty)?;

    let typename = match &*typename {
        "TangleResult" => "TangleResult<_>",
        "TangleResults2" => "TangleResults2<_, _>",
        "TangleResults3" => "TangleResults3<_, _, _>",
        "TangleResults4" => "TangleResults4<_, _, _, _>",
        "TangleResults5" => "TangleResults5<_, _, _, _, _>",
        "TangleResults6" => "TangleResults6<_, _, _, _, _, _>",
        "TangleResults7" => "TangleResults7<_, _, _, _, _, _, _>",
        "TangleResults8" => "TangleResults8<_, _, _, _, _, _, _, _>",
        "TangleResults9" => "TangleResults9<_, _, _, _, _, _, _, _, _>",
        "TangleResults10" => "TangleResults10<_, _, _, _, _, _, _, _, _, _>",
        "TangleResults11" => "TangleResults11<_, _, _, _, _, _, _, _, _, _, _>",
        "TangleResults12" => "TangleResults12<_, _, _, _, _, _, _, _, _, _, _, _>",
        "TangleResults13" => "TangleResults13<_, _, _, _, _, _, _, _, _, _, _, _>",
        "TangleResults14" => "TangleResults14<_, _, _, _, _, _, _, _, _, _, _, _, _>",
        "TangleResults15" => "TangleResults15<_, _, _, _, _, _, _, _, _, _, _, _, _, _>",
        "TangleResults16" => "TangleResults16<_, _, _, _, _, _, _, _, _, _, _, _, _, _, _>",
        "Bytes" => "Bytes",
        "String" => "String",
        _ => return None,
    };

    Some(typename)
}

fn check_output_impls_into_job_result(item_fn: &ItemFn) -> TokenStream {
    let ty = match &item_fn.sig.output {
        syn::ReturnType::Default => return quote! {},
        syn::ReturnType::Type(_, ty) => ty,
    };
    let span = ty.span();

    let declare_inputs = item_fn
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(pat_ty) => {
                let pat = &pat_ty.pat;
                let ty = &pat_ty.ty;
                Some(quote! {
                    let #pat: #ty = panic!();
                })
            }
        })
        .collect::<TokenStream>();

    let block = &item_fn.block;

    let make_value_name = format_ident!(
        "__blueprint_macros_check_{}_into_job_result_make_value",
        item_fn.sig.ident
    );

    let make = if item_fn.sig.asyncness.is_some() {
        quote_spanned! {span=>
            #[allow(warnings)]
            #[allow(unreachable_code)]
            #[doc(hidden)]
            async fn #make_value_name() -> #ty {
                #declare_inputs
                #block
            }
        }
    } else {
        quote_spanned! {span=>
            #[allow(warnings)]
            #[allow(unreachable_code)]
            #[doc(hidden)]
            fn #make_value_name() -> #ty {
                #declare_inputs
                #block
            }
        }
    };

    let name = format_ident!(
        "__blueprint_macros_check_{}_into_job_result",
        item_fn.sig.ident
    );

    if let Some(receiver) = self_receiver(item_fn) {
        quote_spanned! {span=>
            #make

            #[allow(warnings)]
            #[allow(unreachable_code)]
            #[doc(hidden)]
            async fn #name() {
                let value = #receiver #make_value_name().await;
                fn check<T>(_: T)
                    where T: ::blueprint_sdk::job_result::IntoJobResult
                {}
                check(value);
            }
        }
    } else {
        quote_spanned! {span=>
            #[allow(warnings)]
            #[allow(unreachable_code)]
            #[doc(hidden)]
            async fn #name() {
                #make

                let value = #make_value_name().await;

                fn check<T>(_: T)
                where T: ::blueprint_sdk::job_result::IntoJobResult
                {}

                check(value);
            }
        }
    }
}

fn check_future_send(item_fn: &ItemFn, kind: FunctionKind) -> TokenStream {
    if item_fn.sig.asyncness.is_none() {
        match &item_fn.sig.output {
            syn::ReturnType::Default => {
                return syn::Error::new_spanned(
                    item_fn.sig.fn_token,
                    format!("{} must be `async fn`s", kind.name_uppercase_plural()),
                )
                .into_compile_error();
            }
            syn::ReturnType::Type(_, ty) => ty,
        };
    }

    let span = item_fn.sig.ident.span();

    let job_name = &item_fn.sig.ident;

    let args = item_fn.sig.inputs.iter().map(|_| {
        quote_spanned! {span=> panic!() }
    });

    let name = format_ident!("__blueprint_macros_check_{}_future", item_fn.sig.ident);

    let do_check = quote! {
        fn check<T>(_: T)
            where T: ::core::future::Future + Send
        {}
        check(future);
    };

    if let Some(receiver) = self_receiver(item_fn) {
        quote! {
            #[allow(warnings)]
            #[allow(unreachable_code)]
            #[doc(hidden)]
            fn #name() {
                let future = #receiver #job_name(#(#args),*);
                #do_check
            }
        }
    } else {
        quote! {
            #[allow(warnings)]
            #[allow(unreachable_code)]
            #[doc(hidden)]
            fn #name() {
                #item_fn
                let future = #job_name(#(#args),*);
                #do_check
            }
        }
    }
}

fn self_receiver(item_fn: &ItemFn) -> Option<TokenStream> {
    let takes_self = item_fn.sig.inputs.iter().any(|arg| match arg {
        FnArg::Receiver(_) => true,
        FnArg::Typed(typed) => is_self_pat_type(typed),
    });

    if takes_self {
        return Some(quote! { Self:: });
    }

    if let syn::ReturnType::Type(_, ty) = &item_fn.sig.output {
        if let syn::Type::Path(path) = &**ty {
            let segments = &path.path.segments;
            if segments.len() == 1 {
                if let Some(last) = segments.last() {
                    match &last.arguments {
                        syn::PathArguments::None if last.ident == "Self" => {
                            return Some(quote! { Self:: });
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    None
}

/// Given a signature like
///
/// ```skip
/// #[debug_job]
/// async fn job(
///     _: blueprint_sdk::Context<AppContext>,
///     _: Context<AppContext>,
/// ) {}
/// ```
///
/// This will extract `AppContext`.
///
/// Returns `None` if there are no `Context` args or multiple of different types.
fn context_types_from_args(item_fn: &ItemFn) -> HashSet<Type> {
    let types = item_fn
        .sig
        .inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Receiver(_) => None,
            FnArg::Typed(pat_type) => Some(pat_type),
        })
        .map(|pat_type| &*pat_type.ty);
    crate::infer_context_types(types).collect()
}

fn next_is_last_input(item_fn: &ItemFn) -> TokenStream {
    let next_args = item_fn
        .sig
        .inputs
        .iter()
        .enumerate()
        .filter(|(_, arg)| !skip_next_arg(arg, FunctionKind::Middleware))
        .collect::<Vec<_>>();

    if next_args.is_empty() {
        return quote! {
            compile_error!(
                "Middleware functions must take `blueprint_sdk::middleware::Next` as the last argument",
            );
        };
    }

    if next_args.len() == 1 {
        let (idx, arg) = &next_args[0];
        if *idx != item_fn.sig.inputs.len() - 1 {
            return quote_spanned! {arg.span()=>
                compile_error!("`blueprint_sdk::middleware::Next` must be the last argument");
            };
        }
    }

    if next_args.len() >= 2 {
        return quote! {
            compile_error!(
                "Middleware functions can only take one argument of type `blueprint_sdk::middleware::Next`",
            );
        };
    }

    quote! {}
}

fn skip_next_arg(arg: &FnArg, kind: FunctionKind) -> bool {
    match kind {
        FunctionKind::Job => true,
        FunctionKind::Middleware => match arg {
            FnArg::Receiver(_) => true,
            FnArg::Typed(pat_type) => {
                if let Type::Path(type_path) = &*pat_type.ty {
                    type_path
                        .path
                        .segments
                        .last()
                        .is_none_or(|path_segment| path_segment.ident != "Next")
                } else {
                    true
                }
            }
        },
    }
}

#[test]
fn ui_debug_job() {
    crate::run_ui_tests("debug_job");
}
