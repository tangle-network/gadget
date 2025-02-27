//! Job definition traits and implementations for Tangle jobs.
//!
//! This module provides traits and implementations for converting Rust functions into Tangle job
//! definitions that can be registered with a Tangle Services pallet. The main traits are:
//!
//! - [`IntoJobMetadata`]: Converts a type into [`JobMetadata`]
//! - [`IntoJobDefinition`]: Converts a [`Job`] function into a complete [`JobDefinition`]
//! - [`IntoTangleFieldTypes`]: Defines how a type maps to a Tangle [`FieldType`]
//!
//! # Examples
//!
//! ```
//! use blueprint_core::Context;
//! use blueprint_tangle_extra::extract::TangleArg;
//! use blueprint_tangle_extra::extract::TangleResult;
//! use blueprint_tangle_extra::metadata::IntoJobDefinition;
//!
//! // Define a simple squaring job
//! async fn square_job(
//!     Context(_): Context<u64>,
//!     TangleArg(x): TangleArg<u64>,
//! ) -> TangleResult<u64> {
//!     TangleResult(x * x)
//! }
//!
//! // Convert the function into a job definition
//! let job_definition = square_job.into_job_definition();
//! ```
//!
//! [`Job`]: blueprint_core::Job

use gadget_blueprint_serde::{BoundedVec, new_bounded_string};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::FieldType;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::jobs::{
    JobDefinition, JobMetadata,
};

/// Trait for types that can be converted into job metadata.
///
/// This trait is implemented for all types and uses the type name as the job name.
pub trait IntoJobMetadata {
    /// Converts the type into job metadata.
    ///
    /// # Returns
    ///
    /// A [`JobMetadata`] instance with the name set to the type name.
    fn into_job_metadata(self) -> JobMetadata;
}

impl<T> IntoJobMetadata for T {
    fn into_job_metadata(self) -> JobMetadata {
        JobMetadata {
            name: new_bounded_string(core::any::type_name::<T>().to_string()),
            description: None,
        }
    }
}

/// Trait for types that can be converted into a job definition.
///
/// This trait is implemented for functions that can be executed as Tangle jobs.
pub trait IntoJobDefinition<T> {
    /// Converts the implementor into a job definition.
    ///
    /// # Returns
    ///
    /// A [`JobDefinition`] instance with metadata, parameters, and result types.
    fn into_job_definition(self) -> JobDefinition;
}

/// Trait for types that can be converted into Tangle field types.
///
/// This trait defines how a Rust type maps to Tangle field types.
pub trait IntoTangleFieldTypes {
    /// Returns a vector of [`FieldType`] values that represent the type.
    ///
    /// # Returns
    ///
    /// A vector of [`FieldType`] values.
    fn into_tangle_fields() -> Vec<FieldType>;
}

/// Implementation for functions with no arguments.
impl<F, Fut, Res> IntoJobDefinition<((),)> for F
where
    F: FnOnce() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send,
    Res: IntoTangleFieldTypes,
{
    fn into_job_definition(self) -> JobDefinition {
        JobDefinition {
            metadata: self.into_job_metadata(),
            params: BoundedVec(Vec::new()),
            result: BoundedVec(Res::into_tangle_fields()),
        }
    }
}

/// Macro to implement [`IntoJobDefinition`] for functions with arguments.
///
/// This macro generates implementations for functions with different numbers of arguments.
macro_rules! impl_into_job_definition {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        impl<F, Fut, Res, $($ty,)* $last> IntoJobDefinition<((), $($ty,)* $last,)> for F
        where
            F: FnOnce($($ty,)* $last,) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send,
            $last: IntoTangleFieldTypes,
            Res: IntoTangleFieldTypes,
        {
            fn into_job_definition(self) -> JobDefinition {
                JobDefinition {
                    metadata: self.into_job_metadata(),
                    params: BoundedVec($last::into_tangle_fields()),
                    result: BoundedVec(Res::into_tangle_fields()),
                }
            }
        }
    };
}

// Use the all_the_tuples macro from blueprint_core to generate implementations
// for functions with different numbers of arguments.
blueprint_core::all_the_tuples!(impl_into_job_definition);

#[cfg(test)]
mod tests {
    use blueprint_core::Context;

    use super::*;
    use crate::extract::{
        TangleArg, TangleArgs2, TangleArgs3, TangleArgs4, TangleArgs5, TangleArgs6, TangleArgs7,
        TangleArgs8, TangleArgs9, TangleArgs10, TangleArgs11, TangleArgs12, TangleArgs13,
        TangleArgs14, TangleArgs15, TangleArgs16, TangleResult,
    };

    async fn empty() -> TangleResult<u64> {
        TangleResult(0)
    }

    #[test]
    fn empty_test() {
        let empty = empty.into_job_definition();
        assert_eq!(
            empty.metadata.name.0.0,
            b"blueprint_tangle_extra::metadata::job_definition::tests::empty",
            "expected empty, got {}",
            std::str::from_utf8(&empty.metadata.name.0.0).unwrap()
        );
        assert!(empty.params.0.is_empty());
        assert_eq!(empty.result.0[0], FieldType::Uint64);
    }

    #[derive(Debug, Default, Clone, Copy)]
    struct MyContext {
        foo: u64,
    }

    macro_rules! definition_tests {
        ($($func:ident($args_ty:ident($($args:ident),*): $ty:ty));* $(;)?) => {
            $(
                #[allow(unused_variables)]
                async fn $func($args_ty($($args),*): $ty) -> TangleResult<u64> {
                    todo!()
                }

                paste::paste! {
                    #[allow(unused_variables)]
                    async fn [<$func _with_context>](Context(ctx): Context<MyContext>, $args_ty($($args),*): $ty) -> TangleResult<u64> {
                        eprintln!("ctx: {:?}", ctx.foo);
                        todo!()
                    }

                    #[test]
                    fn [<$func _test>]() {
                        let def = $func.into_job_definition();
                        assert_eq!(
                            def.metadata.name.0.0,
                            format!("blueprint_tangle_extra::metadata::job_definition::tests::{}", stringify!($func)).as_bytes(),
                        );
                        assert_eq!(def.params.0.len(), crate::count!($($args),*));
                        assert_eq!(def.result.0.len(), 1);
                        assert!(def.params.0.iter().all(|ty| ty.clone() == FieldType::Uint64));
                        assert_eq!(def.result.0[0], FieldType::Uint64);

                        let def2 = [<$func _with_context>].into_job_definition();
                        assert_eq!(
                            def2.metadata.name.0.0,
                            format!("blueprint_tangle_extra::metadata::job_definition::tests::{}", stringify!([<$func _with_context>])).as_bytes(),
                        );
                        assert_eq!(def2.params.0.len(), crate::count!($($args),*));
                        assert_eq!(def2.result.0.len(), 1);
                        assert!(def2.params.0.iter().all(|ty| ty.clone() == FieldType::Uint64));
                        assert_eq!(def2.result.0[0], FieldType::Uint64);
                    }
                }
            )*
        }
    }

    definition_tests!(
        xsquare(TangleArg(x): TangleArg<u64>);
        xsquare2(TangleArgs2(x, y): TangleArgs2<u64, u64>);
        xsquare3(TangleArgs3(x, y, z): TangleArgs3<u64, u64, u64>);
        xsquare4(TangleArgs4(x, y, z, w): TangleArgs4<u64, u64, u64, u64>);
        xsquare5(TangleArgs5(x, y, z, w, v): TangleArgs5<u64, u64, u64, u64, u64>);
        xsquare6(TangleArgs6(x, y, z, w, v, u): TangleArgs6<u64, u64, u64, u64, u64, u64>);
        xsquare7(TangleArgs7(x, y, z, w, v, u, t): TangleArgs7<u64, u64, u64, u64, u64, u64, u64>);
        xsquare8(TangleArgs8(x, y, z, w, v, u, t, s): TangleArgs8<u64, u64, u64, u64, u64, u64, u64, u64>);
        xsquare9(TangleArgs9(x, y, z, w, v, u, t, s, r): TangleArgs9<u64, u64, u64, u64, u64, u64, u64, u64, u64>);
        xsquare10(TangleArgs10(x, y, z, w, v, u, t, s, r, q): TangleArgs10<u64, u64, u64, u64, u64, u64, u64, u64, u64, u64>);
        xsquare11(TangleArgs11(x, y, z, w, v, u, t, s, r, q, p): TangleArgs11<u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64>);
        xsquare12(TangleArgs12(x, y, z, w, v, u, t, s, r, q, p, o): TangleArgs12<u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64>);
        xsquare13(TangleArgs13(x, y, z, w, v, u, t, s, r, q, p, o, n): TangleArgs13<u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64>);
        xsquare14(TangleArgs14(x, y, z, w, v, u, t, s, r, q, p, o, n, m): TangleArgs14<u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64>);
        xsquare15(TangleArgs15(x, y, z, w, v, u, t, s, r, q, p, o, n, m, l): TangleArgs15<u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64>);
        xsquare16(TangleArgs16(x, y, z, w, v, u, t, s, r, q, p, o, n, m, l, k): TangleArgs16<u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64>);
    );
}
