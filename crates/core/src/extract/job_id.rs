//! Extracts the Job ID from the current job call.

use crate::job_call::Parts as JobCallParts;
use crate::{FromJobCallParts, IntoJobResult};

/// A Specialized extractor for the [`JobId`] of the current [`JobCall`].
///
/// It will try to extract `T` from the job call parts as long as `T` implements [`TryFrom`](core::convert::TryFrom) for [`JobId`] which
/// already works for all the primitive types and can be implemented for custom types.
///
/// [`JobId`]: crate::JobId
/// [`JobCall`]: crate::JobCall
#[derive(Debug, Clone)]
pub struct JobId<T>(pub T);

crate::__impl_deref!(JobId);

impl<T, Ctx> FromJobCallParts<Ctx> for JobId<T>
where
    T: TryFrom<crate::JobId>,
    <T as TryFrom<crate::JobId>>::Error: IntoJobResult,
    Ctx: Send + Sync + 'static,
{
    type Rejection = <T as TryFrom<crate::JobId>>::Error;

    async fn from_job_call_parts(
        parts: &mut JobCallParts,
        _: &Ctx,
    ) -> Result<Self, Self::Rejection> {
        let job_id = parts.job_id;
        T::try_from(job_id).map(JobId)
    }
}

#[cfg(test)]
mod tests {

    use crate::IntoJobId;

    use super::*;

    macro_rules! test_extract_job_id{
        ($($ty:ty),*) => {
            $(
                let job_id: $ty = 42;
                let mut parts = JobCallParts::new(job_id);
                let extracted_job_id: JobId<$ty> = JobId::from_job_call_parts(&mut parts, &()).await.unwrap();
                assert_eq!(extracted_job_id.0, job_id);

                let job_id = <$ty>::MAX;
                let mut parts = JobCallParts::new(job_id);
                let extracted_job_id: JobId<$ty> = JobId::from_job_call_parts(&mut parts, &()).await.unwrap();
                assert_eq!(extracted_job_id.0, job_id);

                let job_id = <$ty>::MIN;
                let mut parts = JobCallParts::new(job_id);
                let extracted_job_id: JobId<$ty> = JobId::from_job_call_parts(&mut parts, &()).await.unwrap();
                assert_eq!(extracted_job_id.0, job_id);
            )*
        };
    }

    #[tokio::test]
    async fn it_correctly_extracts_all_numbers() {
        test_extract_job_id!(
            u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, i128, u128
        );
    }

    #[derive(Copy, Clone, Debug, PartialEq)]
    struct MyCustomJobId;

    impl TryFrom<crate::JobId> for MyCustomJobId {
        type Error = ();

        fn try_from(job_id: crate::JobId) -> Result<Self, Self::Error> {
            if u64::from(job_id) == 42u64 {
                Ok(MyCustomJobId)
            } else {
                Err(())
            }
        }
    }

    impl IntoJobId for MyCustomJobId {
        fn into_job_id(self) -> crate::JobId {
            42u64.into()
        }
    }

    #[tokio::test]
    async fn it_works_for_custom_types() {
        let job_id: MyCustomJobId = MyCustomJobId;
        let mut parts = JobCallParts::new(job_id);

        let extracted_job_id: JobId<MyCustomJobId> =
            JobId::from_job_call_parts(&mut parts, &()).await.unwrap();
        assert_eq!(extracted_job_id.0, job_id);
    }

    #[tokio::test]
    async fn it_rejects_invalid_job_ids() {
        let mut parts = JobCallParts::new(41u64);
        let extracted_job_id = JobId::<MyCustomJobId>::from_job_call_parts(&mut parts, &()).await;
        assert!(extracted_job_id.is_err());
    }
}
