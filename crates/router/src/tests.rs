extern crate std;

use crate::Router;
use crate::test_helpers::setup_log;
use blueprint_core::error::BoxError;
use blueprint_core::job_call::JobCall;
use bytes::Bytes;
use tower::Service;

#[tokio::test]
async fn fallible_job() {
    setup_log();

    async fn job_ok() -> Result<(), BoxError> {
        Ok(())
    }

    async fn job_err() -> Result<(), BoxError> {
        Err("error".into())
    }

    let mut router: Router = Router::new().route(0, job_ok).route(1, job_err);

    let Some(res) = router.call(JobCall::new(0, Bytes::new())).await.unwrap() else {
        panic!("job should produce a result");
    };

    std::assert_eq!(res.len(), 1);
    std::assert!(res[0].is_ok());

    let Some(res) = router.call(JobCall::new(1, Bytes::new())).await.unwrap() else {
        panic!("job should produce a result");
    };

    std::assert_eq!(res.len(), 1);
    std::assert!(res[0].is_err());
}
