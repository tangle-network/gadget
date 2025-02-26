use blueprint_core::IntoJobResult;
use blueprint_core::JobResult;
use bytes::Bytes;
use gadget_blueprint_serde::Field;
use gadget_blueprint_serde::to_field;
use tangle_subxt::parity_scale_codec::Encode;
use tangle_subxt::subxt::utils::AccountId32;

use blueprint_core::__define_rejection as define_rejection;

define_rejection! {
  #[body = "Failed to convert the job result into a tangle result"]
  /// A Rejection type for [`TangleResult`] when it fails to be converted into a job result.
  pub struct IntoJobResultFailed(Error);
}

#[rustfmt::skip]
macro_rules! all_the_tuples {
    ($name:ident) => {
        $name!(TangleResult, T1);
        $name!(TangleResults2, T1, T2);
        $name!(TangleResults3, T1, T2, T3);
        $name!(TangleResults4, T1, T2, T3, T4);
        $name!(TangleResults5, T1, T2, T3, T4, T5);
        $name!(TangleResults6, T1, T2, T3, T4, T5, T6);
        $name!(TangleResults7, T1, T2, T3, T4, T5, T6, T7);
        $name!(TangleResults8, T1, T2, T3, T4, T5, T6, T7, T8);
        $name!(TangleResults9, T1, T2, T3, T4, T5, T6, T7, T8, T9);
        $name!(TangleResults10, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
        $name!(TangleResults11, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
        $name!(TangleResults12, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
        $name!(TangleResults13, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
        $name!(TangleResults14, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
        $name!(TangleResults15, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
        $name!(TangleResults16, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
    };
}

macro_rules! impl_tangle_results {
    (
        $name:ident,
        $($ty:ident),*
    ) => {
        /// A simple wrapper that converts the result of a job call into a tangle specific result.
        ///
        /// This will work for any type that implements [`serde::Serialize`].
        #[derive(Debug, Clone)]
        pub struct $name<$($ty,)*>($(pub $ty,)*);

        #[allow(non_snake_case, unused_mut)]
        impl<$($ty,)*> IntoJobResult for $name<$($ty,)*>
        where
            $( $ty: serde::Serialize, )*
        {
            fn into_job_result(self) -> Option<JobResult> {
                let mut fields: Vec::<Field<AccountId32>> = Vec::with_capacity(16);
                let $name($($ty,)*) = self;
                $(
                    match to_field($ty) {
                        Ok(field) => fields.push(field),
                        Err(e) => return IntoJobResultFailed::from_err(e).into_job_result(),
                    };
                )*

                // Encode the fields into a `Bytes` object.
                Bytes::from(fields.encode()).into_job_result()
            }
        }
    };
}

all_the_tuples!(impl_tangle_results);
