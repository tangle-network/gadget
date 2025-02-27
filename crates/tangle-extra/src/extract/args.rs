use blueprint_core::{FromJobCall, JobCall};

use blueprint_core::__composite_rejection as composite_rejection;
use blueprint_core::__define_rejection as define_rejection;

use gadget_blueprint_serde::{Field, from_field};
use tangle_subxt::FieldExt;
use tangle_subxt::parity_scale_codec::Decode;
use tangle_subxt::subxt::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::FieldType;

define_rejection! {
    #[body = "Missing argument in the job call"]
    /// A Rejection type for [`TangleArg`] when it fails to extract the arguments from the job call.
    pub struct MissingArgument(Error);
}

define_rejection! {
  #[body = "Failed to extract the arguments from the job call"]
  /// A Rejection type for [`TangleArg`] when it fails to extract the arguments from the job call.
  pub struct ArgsExtractionFailed(Error);
}

composite_rejection! {
    /// Rejection used for [`TangleArg`].
    ///
    /// Contains one variant for each way the [`TangleArg`] extractor
    /// can fail.
    pub enum TangleArgsRejection {
        MissingArgument,
        ArgsExtractionFailed,
    }
}

#[rustfmt::skip]
macro_rules! all_the_tuples {
    ($name:ident) => {
        $name!(TangleArg, T1);
        $name!(TangleArgs2, T1, T2);
        $name!(TangleArgs3, T1, T2, T3);
        $name!(TangleArgs4, T1, T2, T3, T4);
        $name!(TangleArgs5, T1, T2, T3, T4, T5);
        $name!(TangleArgs6, T1, T2, T3, T4, T5, T6);
        $name!(TangleArgs7, T1, T2, T3, T4, T5, T6, T7);
        $name!(TangleArgs8, T1, T2, T3, T4, T5, T6, T7, T8);
        $name!(TangleArgs9, T1, T2, T3, T4, T5, T6, T7, T8, T9);
        $name!(TangleArgs10, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
        $name!(TangleArgs11, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
        $name!(TangleArgs12, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
        $name!(TangleArgs13, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
        $name!(TangleArgs14, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
        $name!(TangleArgs15, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
        $name!(TangleArgs16, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
    };
}

macro_rules! impl_tangle_job_args {
    (
        $name:ident,
        $($ty:ident),*
    ) => {
        /// An extractor for the arguments of a job call that deserializes the arguments from the job call body.
        ///
        /// This will work for any type that implements [`serde::de::DeserializeOwned`].
        #[derive(Debug, Clone)]
        pub struct $name<$($ty,)*>($(pub $ty,)*);

        #[allow(non_snake_case, unused_mut)]
        impl<Ctx, $($ty,)*> FromJobCall<Ctx> for $name<$($ty,)*>
        where
            Ctx: Send + Sync,
            $( $ty: serde::de::DeserializeOwned, )*
        {
            type Rejection = TangleArgsRejection;

            async fn from_job_call(call: JobCall, _ctx: &Ctx) -> Result<Self, Self::Rejection> {
                let fields = Vec::<Field<AccountId32>>::decode(&mut call.body().as_ref())
                    .map_err(ArgsExtractionFailed::from_err)?;
                let mut args = fields.into_iter();
                $(
                    let $ty = match args.next() {
                        Some(field) => from_field(field).map_err(ArgsExtractionFailed::from_err)?,
                        None => return Err(MissingArgument::from_err(stringify!($ty)).into())
                    };
                )*

                Ok($name($($ty,)*))
            }
        }
    };
}

macro_rules! impl_tangle_field_types {
    (
        $name:ident,
        $($ty:ident),*
    ) => {
        impl<$($ty,)*> crate::metadata::IntoTangleFieldTypes for $name<$($ty,)*>
        where
            $($ty: core::default::Default + serde::Serialize,)*
        {
            fn into_tangle_fields() -> Vec<FieldType> {
                let mut ret = Vec::with_capacity(crate::count!($($ty,)*));
                $(
                    ret.push(gadget_blueprint_serde::to_field(<$ty as core::default::Default>::default()).expect("type should serialize").field_type());
                )*
                ret
            }
        }
    }
}

all_the_tuples!(impl_tangle_job_args);
all_the_tuples!(impl_tangle_field_types);
