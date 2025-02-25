pub mod config;
pub mod error;

use blueprint_core::{BoxError, JobCall};
use blueprint_router::Router;
use config::GadgetConfiguration;
use core::pin::Pin;
use error::RunnerError as Error;
use futures::Future;
use futures_core::Stream;
use futures_util::{StreamExt, TryStreamExt};
use std::future;
use std::future::poll_fn;
use tokio::sync::oneshot;
use tower::Service;

pub trait CloneableConfig: Send + Sync {
    fn clone_box(&self) -> Box<dyn BlueprintConfig>;
}

impl<T> CloneableConfig for T
where
    T: BlueprintConfig + Clone,
{
    fn clone_box(&self) -> Box<dyn BlueprintConfig> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
pub trait BlueprintConfig: Send + Sync + CloneableConfig + 'static {
    async fn register(&self, _env: &GadgetConfiguration) -> Result<(), Error> {
        Ok(())
    }

    async fn requires_registration(&self, _env: &GadgetConfiguration) -> Result<bool, Error> {
        Ok(true)
    }
    /// Controls whether the runner should exit after registration
    ///
    /// Returns true if the runner should exit after registration, false if it should continue
    fn should_exit_after_registration(&self) -> bool {
        true // By default, runners exit after registration
    }
}

impl BlueprintConfig for () {}

// Clone traits with distinct names
pub trait CloneableService: Send {
    fn clone_box(&self) -> Box<dyn BackgroundService>;
}

impl<T> CloneableService for T
where
    T: BackgroundService + Clone,
{
    fn clone_box(&self) -> Box<dyn BackgroundService> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
pub trait BackgroundService: Send + Sync + CloneableService + 'static {
    async fn start(&self) -> Result<oneshot::Receiver<Result<(), Error>>, Error>;
}

type Producer = Box<dyn Stream<Item = Result<JobCall, BoxError>> + Send + Unpin + 'static>;

pub struct BlueprintRunnerBuilder<F> {
    config: Box<dyn BlueprintConfig>,
    env: GadgetConfiguration,
    producers: Vec<Producer>,
    router: Option<Router>,
    background_services: Vec<Box<dyn BackgroundService>>,
    shutdown_handler: F,
}

impl<F> BlueprintRunnerBuilder<F>
where
    F: Future<Output = ()> + Send + 'static,
{
    pub fn router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    pub fn producer<E>(
        mut self,
        producer: impl Stream<Item = Result<JobCall, E>> + Send + Unpin + 'static,
    ) -> Self
    where
        E: Into<BoxError>,
    {
        self.producers
            .push(Box::new(producer.map_err(|e| e.into())));
        self
    }

    pub fn background_service(mut self, service: impl BackgroundService) -> Self {
        self.background_services.push(Box::new(service));
        self
    }

    pub fn with_shutdown_handler<F2>(self, handler: F2) -> BlueprintRunnerBuilder<F2>
    where
        F2: Future<Output = ()> + Send + 'static,
    {
        BlueprintRunnerBuilder {
            config: self.config,
            env: self.env,
            producers: self.producers,
            router: self.router,
            background_services: self.background_services,
            shutdown_handler: handler,
        }
    }

    pub async fn run(self) -> Result<(), Error> {
        let Some(router) = self.router else {
            return Err(Error::NoRouter);
        };

        let runner = FinalizedBlueprintRunner {
            config: self.config,
            producers: self.producers,
            router,
            env: self.env,
            background_services: self.background_services,
            shutdown_handler: self.shutdown_handler,
        };

        runner.run().await
    }
}

pub struct BlueprintRunner;

impl BlueprintRunner {
    pub fn new<C: BlueprintConfig + 'static>(
        config: C,
        env: GadgetConfiguration,
    ) -> BlueprintRunnerBuilder<impl Future<Output = ()> + Send + 'static> {
        BlueprintRunnerBuilder {
            config: Box::new(config),
            env,
            producers: Vec::new(),
            router: None,
            background_services: Vec::new(),
            shutdown_handler: future::pending(),
        }
    }
}

struct FinalizedBlueprintRunner<F> {
    config: Box<dyn BlueprintConfig>,
    producers: Vec<Producer>,
    router: Router,
    env: GadgetConfiguration,
    background_services: Vec<Box<dyn BackgroundService>>,
    shutdown_handler: F,
}

impl<F> FinalizedBlueprintRunner<F>
where
    F: Future<Output = ()> + Send + 'static,
{
    async fn run(self) -> Result<(), Error> {
        if self.config.requires_registration(&self.env).await? {
            self.config.register(&self.env).await?;
            if self.config.should_exit_after_registration() {
                return Ok(());
            }
        }

        // TODO: Config is unused
        let FinalizedBlueprintRunner {
            config: _,
            producers,
            mut router,
            env: _,
            background_services,
            shutdown_handler,
        } = self;

        let mut router = router.as_service();

        let mut background_receivers = Vec::with_capacity(background_services.len());
        for service in background_services {
            let receiver = service.start().await?;
            background_receivers.push(receiver);
        }

        let mut background_futures = Vec::with_capacity(background_receivers.len());

        // Startup background services
        for receiver in background_receivers {
            background_futures.push(Box::pin(async move {
                receiver
                    .await
                    .map_err(|e| Error::BackgroundService(e.to_string()))
                    .and(Ok(()))
            })
                as Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>);
        }

        let (mut shutdown_tx, shutdown_rx) = oneshot::channel();
        tokio::spawn(async move {
            let _ = shutdown_rx.await;
            tracing::info!("Received graceful shutdown signal. Calling shutdown handler");
            shutdown_handler.await;
        });

        poll_fn(|ctx| router.poll_ready(ctx)).await.unwrap_or(());

        let mut producer_stream = futures::stream::select_all(producers);
        let mut background_services = if background_futures.is_empty() {
            futures::future::select_all(vec![Box::pin(futures::future::ready(Ok(())))
                as Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>])
        } else {
            futures::future::select_all(background_futures)
        };

        // TODO: Stop using string errors
        loop {
            tokio::select! {
                producer_result = producer_stream.next() => {
                    match producer_result {
                        Some(Ok(job_call)) => {
                            match router.call(job_call).await {
                                Ok(_results) => {
                                    tracing::warn!("TODO: Pass the results to the consumers");
                                },
                                Err(e) => {
                                    tracing::error!("Job call failed: {:?}", e);
                                    let _ = shutdown_tx.send(true);
                                    return Err(Error::JobCall(e.to_string()));
                                }
                            }
                        }
                        Some(Err(e)) => {
                            tracing::error!("Producer error: {:?}", e);
                            let _ = shutdown_tx.send(true);
                            return Err(Error::JobCall(e.to_string()));
                        }
                        None => {
                            tracing::error!("Producer stream ended unexpectedly");
                            let _ = shutdown_tx.send(true);
                            return Err(Error::JobCall("Producer stream ended".into()));
                        }
                    }
                }
                result = &mut background_services => {
                    let (result, _, remaining_background_services) = result;
                    match result {
                        Ok(()) => {
                            tracing::warn!("A background service has finished running");
                        },
                        Err(e) => {
                            tracing::error!("A background service failed: {:?}", e);
                            let _ = shutdown_tx.send(true);
                            return Err(e);
                        }
                    }

                    if remaining_background_services.is_empty() {
                        tracing::warn!("All background services have ended");
                        continue;
                    }

                    background_services = futures::future::select_all(remaining_background_services);
                }
                _ = shutdown_tx.closed() => {
                    break
                }
            }
        }

        Ok(())
    }
}
