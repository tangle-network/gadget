use crate::job::error::JobError;
use crate::job::{ExecutableJobWrapper, ProceedWithExecution, SendFuture};
use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

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
