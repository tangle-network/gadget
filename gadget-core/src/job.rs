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

    async fn execute(&mut self) -> Result<(), JobError> {
        match self.pre_job_hook().await? {
            ProceedWithExecution::True => {
                let result = self.job().await;
                let post_result = self.post_job_hook().await;
                result.and(post_result)
            }
            ProceedWithExecution::False => Ok(()),
        }
    }
}

pub struct ExecutableJobWrapper<Pre: ?Sized, Protocol: ?Sized, Post: ?Sized> {
    pre: Pin<Box<Pre>>,
    protocol: Pin<Box<Protocol>>,
    post: Pin<Box<Post>>,
}

#[async_trait]
impl<Pre: ?Sized, Protocol: ?Sized, Post: ?Sized> ExecutableJob
    for ExecutableJobWrapper<Pre, Protocol, Post>
where
    Pre: SendFuture<'static, Result<ProceedWithExecution, JobError>>,
    Protocol: SendFuture<'static, Result<(), JobError>>,
    Post: SendFuture<'static, Result<(), JobError>>,
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
}

impl<Pre, Protocol, Post> ExecutableJobWrapper<Pre, Protocol, Post>
where
    Pre: SendFuture<'static, Result<ProceedWithExecution, JobError>>,
    Protocol: SendFuture<'static, Result<(), JobError>>,
    Post: SendFuture<'static, Result<(), JobError>>,
{
    pub fn new(pre: Pre, protocol: Protocol, post: Post) -> Self {
        Self {
            pre: Box::pin(pre),
            protocol: Box::pin(protocol),
            post: Box::pin(post),
        }
    }
}

#[derive(Default)]
pub struct JobBuilder {
    pre: Option<Pin<Box<PreJobHook>>>,
    protocol: Option<Pin<Box<dyn SendFuture<'static, Result<(), JobError>>>>>,
    post: Option<Pin<Box<PostJobHook>>>,
}

pub type PreJobHook = dyn SendFuture<'static, Result<ProceedWithExecution, JobError>>;
pub type PostJobHook = dyn SendFuture<'static, Result<(), JobError>>;

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

pub type BuiltExecutableJobWrapper = ExecutableJobWrapper<
    dyn SendFuture<'static, Result<ProceedWithExecution, JobError>>,
    dyn SendFuture<'static, Result<(), JobError>>,
    dyn SendFuture<'static, Result<(), JobError>>,
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

        let protocol = Box::pin(self.protocol.expect("Must specify protocol"));

        ExecutableJobWrapper {
            pre,
            protocol,
            post,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::job::ExecutableJob;

    #[tokio::test]
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

        let mut job = super::ExecutableJobWrapper::new(pre, protocol, post);
        job.execute().await.unwrap();
        assert_eq!(counter_final.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
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

        let mut job = super::ExecutableJobWrapper::new(pre, protocol, post);
        job.execute().await.unwrap();
        assert_eq!(counter_final.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
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
}
