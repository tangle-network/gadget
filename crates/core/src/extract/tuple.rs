use super::{FromJobCall, FromJobCallParts, JobCall};
use crate::JobResult;
use crate::job_call::Parts;
use crate::job_result::IntoJobResult;
use core::convert::Infallible;

impl<Ctx> FromJobCallParts<Ctx> for ()
where
    Ctx: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_job_call_parts(_: &mut Parts, _: &Ctx) -> Result<(), Self::Rejection> {
        Ok(())
    }
}

macro_rules! impl_from_job_call {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<Ctx, $($ty,)* $last> FromJobCallParts<Ctx> for ($($ty,)* $last,)
        where
            $( $ty: FromJobCallParts<Ctx> + Send, )*
            $last: FromJobCallParts<Ctx> + Send,
            Ctx: Send + Sync,
        {
            type Rejection = JobResult;

            async fn from_job_call_parts(parts: &mut Parts, ctx: &Ctx) -> Result<Self, Self::Rejection> {
                $(
                    let $ty = $ty::from_job_call_parts(parts, ctx)
                        .await
                        .map_err(|err| err.into_job_result())?;
                )*
                let $last = $last::from_job_call_parts(parts, ctx)
                    .await
                    .map_err(|err| err.into_job_result())?;

                Ok(($($ty,)* $last,))
            }
        }

        // This impl must not be generic over M, otherwise it would conflict with the blanket
        // implementation of `FromJobCall<S, Mut>` for `T: FromJobCallParts<S>`.
        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<Ctx, $($ty,)* $last> FromJobCall<Ctx> for ($($ty,)* $last,)
        where
            $( $ty: FromJobCallParts<Ctx> + Send, )*
            $last: FromJobCall<Ctx> + Send,
            Ctx: Send + Sync,
        {
            type Rejection = JobResult;

            async fn from_job_call(req: JobCall, ctx: &Ctx) -> Result<Self, Self::Rejection> {
                let (mut parts, body) = req.into_parts();

                $(
                    let $ty = $ty::from_job_call_parts(&mut parts, ctx).await.map_err(|err| err.into_job_result())?;
                )*

                let req = JobCall::from_parts(parts, body);

                let $last = $last::from_job_call(req, ctx).await.map_err(|err| err.into_job_result())?;

                Ok(($($ty,)* $last,))
            }
        }
    };
}

all_the_tuples!(impl_from_job_call);

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::extract::{FromJobCall, FromJobCallParts};
    use crate::job_call::Parts as JobCallParts;

    struct JobId;

    impl<Ctx: Sync> FromJobCallParts<Ctx> for JobId {
        type Rejection = ();
        async fn from_job_call_parts(
            _: &mut JobCallParts,
            _: &Ctx,
        ) -> Result<Self, Self::Rejection> {
            Ok(Self)
        }
    }

    fn assert_from_job_call<M, T>()
    where
        T: FromJobCall<(), M>,
    {
    }

    fn assert_from_job_call_parts<T: FromJobCallParts<()>>() {}

    #[test]
    fn unit() {
        assert_from_job_call_parts::<()>();
        assert_from_job_call::<_, ()>();
    }

    #[test]
    fn tuple_of_one() {
        assert_from_job_call_parts::<(JobId,)>();
        assert_from_job_call::<_, (JobId,)>();
        assert_from_job_call::<_, (Bytes,)>();
    }

    #[test]
    fn tuple_of_two() {
        assert_from_job_call_parts::<((), ())>();
        assert_from_job_call::<_, ((), ())>();
        assert_from_job_call::<_, (JobId, Bytes)>();
    }

    #[test]
    fn nested_tuple() {
        assert_from_job_call_parts::<(((JobId,),),)>();
        assert_from_job_call::<_, ((((Bytes,),),),)>();
    }
}
