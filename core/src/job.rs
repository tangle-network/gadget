use crate::job_manager::SendFuture;
use async_trait::async_trait;
use std::error::Error;
use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub enum ProceedWithExecution {
    True,
    False,
}

#[derive(Debug)]
pub struct JobError {
    pub reason: String,
}

impl<T: Into<String>> From<T> for JobError {
    fn from(value: T) -> Self {
        Self {
            reason: value.into(),
        }
    }
}

impl Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{reason}", reason = self.reason)
    }
}

impl Error for JobError {}

#[async_trait]
pub trait ExecutableJob: Send + 'static {
    async fn pre_job_hook(&mut self) -> Result<ProceedWithExecution, JobError>;
    async fn job(&mut self) -> Result<(), JobError>;
    async fn post_job_hook(&mut self) -> Result<(), JobError>;
    async fn catch(&mut self);

    async fn execute(&mut self) -> Result<(), JobError> {
        match self.pre_job_hook().await? {
            ProceedWithExecution::True => match self.job().await {
                Ok(_) => self.post_job_hook().await,
                Err(err) => {
                    self.catch().await;
                    Err(err)
                }
            },
            ProceedWithExecution::False => Ok(()),
        }
    }
}

pub struct ExecutableJobWrapper<Pre: ?Sized, Protocol: ?Sized, Post: ?Sized, Catch: ?Sized> {
    pre: Pin<Box<Pre>>,
    protocol: Pin<Box<Protocol>>,
    post: Pin<Box<Post>>,
    catch: Pin<Box<Catch>>,
}

#[async_trait]
impl<Pre: ?Sized, Protocol: ?Sized, Post: ?Sized, Catch: ?Sized> ExecutableJob
    for ExecutableJobWrapper<Pre, Protocol, Post, Catch>
where
    Pre: SendFuture<'static, Result<ProceedWithExecution, JobError>>,
    Protocol: SendFuture<'static, Result<(), JobError>>,
    Post: SendFuture<'static, Result<(), JobError>>,
    Catch: SendFuture<'static, ()>,
{
    async fn pre_job_hook(&mut self) -> Result<ProceedWithExecution, JobError> {
        self.pre.as_mut().await
    }

    async fn job(&mut self) -> Result<(), JobError> {
        self.protocol.as_mut().await
    }

    async fn post_job_hook(&mut self) -> Result<(), JobError> {
        self.post.as_mut().await
    }

    async fn catch(&mut self) {
        self.catch.as_mut().await
    }
}

impl<Pre, Protocol, Post, Catch> ExecutableJobWrapper<Pre, Protocol, Post, Catch>
where
    Pre: SendFuture<'static, Result<ProceedWithExecution, JobError>>,
    Protocol: SendFuture<'static, Result<(), JobError>>,
    Post: SendFuture<'static, Result<(), JobError>>,
    Catch: SendFuture<'static, ()>,
{
    pub fn new(pre: Pre, protocol: Protocol, post: Post, catch: Catch) -> Self {
        Self {
            pre: Box::pin(pre),
            protocol: Box::pin(protocol),
            post: Box::pin(post),
            catch: Box::pin(catch),
        }
    }
}

#[derive(Default)]
pub struct JobBuilder {
    pre: Option<Pin<Box<PreJobHook>>>,
    protocol: Option<Pin<Box<ProtocolJobHook>>>,
    post: Option<Pin<Box<PostJobHook>>>,
    catch: Option<Pin<Box<CatchJobHook>>>,
}

pub type PreJobHook = dyn SendFuture<'static, Result<ProceedWithExecution, JobError>>;
pub type PostJobHook = dyn SendFuture<'static, Result<(), JobError>>;
pub type ProtocolJobHook = dyn SendFuture<'static, Result<(), JobError>>;
pub type CatchJobHook = dyn SendFuture<'static, ()>;

pub struct DefaultPreJobHook;
impl Future for DefaultPreJobHook {
    type Output = Result<ProceedWithExecution, JobError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(Ok(ProceedWithExecution::True))
    }
}

pub struct DefaultPostJobHook;
impl Future for DefaultPostJobHook {
    type Output = Result<(), JobError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(Ok(()))
    }
}

struct DefaultCatchJobHook;

impl Future for DefaultCatchJobHook {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

pub type BuiltExecutableJobWrapper = ExecutableJobWrapper<
    dyn SendFuture<'static, Result<ProceedWithExecution, JobError>>,
    dyn SendFuture<'static, Result<(), JobError>>,
    dyn SendFuture<'static, Result<(), JobError>>,
    dyn SendFuture<'static, ()>,
>;

impl JobBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pre<Pre>(mut self, pre: Pre) -> Self
    where
        Pre: SendFuture<'static, Result<ProceedWithExecution, JobError>>,
    {
        self.pre = Some(Box::pin(pre));
        self
    }

    pub fn protocol<Protocol>(mut self, protocol: Protocol) -> Self
    where
        Protocol: SendFuture<'static, Result<(), JobError>>,
    {
        self.protocol = Some(Box::pin(protocol));
        self
    }

    pub fn post<Post>(mut self, post: Post) -> Self
    where
        Post: SendFuture<'static, Result<(), JobError>>,
    {
        self.post = Some(Box::pin(post));
        self
    }

    pub fn catch<Catch>(mut self, catch: Catch) -> Self
    where
        Catch: SendFuture<'static, ()>,
    {
        self.catch = Some(Box::pin(catch));
        self
    }

    pub fn build(self) -> BuiltExecutableJobWrapper {
        let pre = if let Some(pre) = self.pre {
            pre
        } else {
            Box::pin(DefaultPreJobHook)
        };

        let post = if let Some(post) = self.post {
            post
        } else {
            Box::pin(DefaultPostJobHook)
        };

        let catch = if let Some(catch) = self.catch {
            catch
        } else {
            Box::pin(DefaultCatchJobHook)
        };

        let protocol = Box::pin(self.protocol.expect("Must specify protocol"));

        ExecutableJobWrapper {
            pre,
            protocol,
            post,
            catch,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::job::ExecutableJob;

    #[cfg(target_family = "wasm")]
    use wasm_bindgen_test::*;
    #[cfg(target_family = "wasm")]
    wasm_bindgen_test_configure!(run_in_browser);

    #[tokio::test]
    #[wasm_bindgen_test]
    async fn test_executable_job_wrapper_proceed() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let counter_clone2 = counter.clone();
        let counter_final = counter.clone();

        let pre = async move {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(super::ProceedWithExecution::True)
        };

        let protocol = async move {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        };

        let post = async move {
            counter_clone2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        };

        let catch = async move {};

        let mut job = super::ExecutableJobWrapper::new(pre, protocol, post, catch);
        job.execute().await.unwrap();
        assert_eq!(counter_final.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    #[wasm_bindgen_test]
    async fn test_executable_job_wrapper_no_proceed() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let counter_clone2 = counter.clone();
        let counter_final = counter.clone();

        let pre = async move {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(super::ProceedWithExecution::False)
        };

        let protocol = async move {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        };

        let post = async move {
            counter_clone2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        };

        let catch = async move {};

        let mut job = super::ExecutableJobWrapper::new(pre, protocol, post, catch);
        job.execute().await.unwrap();
        assert_eq!(counter_final.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    #[wasm_bindgen_test]
    async fn test_job_builder() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let counter_clone2 = counter.clone();
        let counter_final = counter.clone();

        let mut job = super::JobBuilder::new()
            .pre(async move {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(super::ProceedWithExecution::True)
            })
            .protocol(async move {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .post(async move {
                counter_clone2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .build();

        job.execute().await.unwrap();
        assert_eq!(counter_final.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    #[wasm_bindgen_test]
    async fn test_job_builder_no_pre() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let counter_clone2 = counter.clone();
        let counter_final = counter.clone();

        let mut job = super::JobBuilder::default()
            .protocol(async move {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .post(async move {
                counter_clone2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .build();

        job.execute().await.unwrap();
        assert_eq!(counter_final.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    #[wasm_bindgen_test]
    async fn test_job_builder_no_post() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let counter_final = counter.clone();

        let mut job = super::JobBuilder::default()
            .pre(async move {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(super::ProceedWithExecution::True)
            })
            .protocol(async move {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .build();

        job.execute().await.unwrap();
        assert_eq!(counter_final.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    #[wasm_bindgen_test]
    async fn test_job_builder_no_pre_no_post() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let counter_final = counter.clone();

        let mut job = super::JobBuilder::default()
            .protocol(async move {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .build();

        job.execute().await.unwrap();
        assert_eq!(counter_final.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    #[wasm_bindgen_test]
    async fn test_protocol_err_catch_performs_increment() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let counter_clone2 = counter.clone();
        let counter_final = counter.clone();

        let pre = async move {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(super::ProceedWithExecution::True)
        };

        let protocol = async move {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(super::JobError::from("Protocol error"))
        };

        let post = async move { unreachable!("Post should not be called") };

        let catch = async move {
            counter_clone2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        };

        let mut job = super::ExecutableJobWrapper::new(pre, protocol, post, catch);
        job.execute().await.unwrap_err();
        assert_eq!(counter_final.load(std::sync::atomic::Ordering::SeqCst), 3);
    }
}
