extern crate alloc;

pub mod config;
pub mod error;

#[cfg(feature = "eigenlayer")]
pub mod eigenlayer;
#[cfg(feature = "symbiotic")]
mod symbiotic;
#[cfg(feature = "tangle")]
pub mod tangle;

use blueprint_core::{BoxError, JobCall, JobResult};
use blueprint_router::Router;
use config::GadgetConfiguration;
use core::future::{self, poll_fn};
use core::pin::Pin;
use error::RunnerError as Error;
use futures::{Future, Sink};
use futures_core::Stream;
use futures_util::{SinkExt, StreamExt, TryStreamExt, stream};
use tokio::sync::oneshot;
use tower::Service;

#[allow(async_fn_in_trait)]
#[dynosaur::dynosaur(DynBlueprintConfig)]
pub trait BlueprintConfig: Send + Sync {
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

unsafe impl Send for DynBlueprintConfig<'_> {}
unsafe impl Sync for DynBlueprintConfig<'_> {}

impl BlueprintConfig for () {}

#[allow(async_fn_in_trait)]
#[dynosaur::dynosaur(DynBackgroundService)]
pub trait BackgroundService: Send + Sync {
    async fn start(&self) -> Result<oneshot::Receiver<Result<(), Error>>, Error>;
}

unsafe impl Send for DynBackgroundService<'_> {}
unsafe impl Sync for DynBackgroundService<'_> {}

type Producer = Box<dyn Stream<Item = Result<JobCall, BoxError>> + Send + Unpin + 'static>;
type Consumer = Box<dyn Sink<JobResult, Error = BoxError> + Send + Unpin + 'static>;

pub struct BlueprintRunnerBuilder<F> {
    config: Box<DynBlueprintConfig<'static>>,
    env: GadgetConfiguration,
    producers: Vec<Producer>,
    consumers: Vec<Consumer>,
    router: Option<Router>,
    background_services: Vec<Box<DynBackgroundService<'static>>>,
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

    pub fn consumer<E>(
        mut self,
        consumer: impl Sink<JobResult, Error = E> + Send + Unpin + 'static,
    ) -> Self
    where
        E: Into<BoxError>,
    {
        self.consumers
            .push(Box::new(consumer.sink_map_err(|e| e.into())));
        self
    }

    pub fn background_service(mut self, service: impl BackgroundService + 'static) -> Self {
        self.background_services.push(DynBackgroundService::boxed(service));
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
            consumers: self.consumers,
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
            consumers: self.consumers,
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
    pub fn builder<C: BlueprintConfig + 'static>(
        config: C,
        env: GadgetConfiguration,
    ) -> BlueprintRunnerBuilder<impl Future<Output = ()> + Send + 'static> {
        BlueprintRunnerBuilder {
            config: DynBlueprintConfig::boxed(config),
            env,
            producers: Vec::new(),
            consumers: Vec::new(),
            router: None,
            background_services: Vec::new(),
            shutdown_handler: future::pending(),
        }
    }
}

struct FinalizedBlueprintRunner<F> {
    config: Box<DynBlueprintConfig<'static>>,
    producers: Vec<Producer>,
    consumers: Vec<Consumer>,
    router: Router,
    env: GadgetConfiguration,
    background_services: Vec<Box<DynBackgroundService<'static>>>,
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
            mut consumers,
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
            blueprint_core::info!("Received graceful shutdown signal. Calling shutdown handler");
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
                                Ok(Some(results)) => {
                                    let result_stream = stream::iter(results.into_iter().map(Ok));

                                    // Broadcast results to all consumers
                                    let send_futures = consumers.iter_mut().map(|consumer| {
                                        let mut stream_clone = result_stream.clone();
                                        async move {
                                            consumer.send_all(&mut stream_clone).await
                                        }
                                    });

                                    let result = futures::future::try_join_all(send_futures).await;
                                    if let Err(e) = result {
                                        let _ = shutdown_tx.send(true);
                                        return Err(Error::Consumer(e));
                                    }
                                },
                                Ok(None) => {
                                    blueprint_core::debug!("Job call was ignored by router");
                                },
                                Err(e) => {
                                    blueprint_core::error!("Job call failed: {:?}", e);
                                    let _ = shutdown_tx.send(true);
                                    return Err(Error::JobCall(e.to_string()));
                                },
                            }
                        }
                        Some(Err(e)) => {
                            blueprint_core::error!("Producer error: {:?}", e);
                            let _ = shutdown_tx.send(true);
                            return Err(Error::JobCall(e.to_string()));
                        }
                        None => {
                            blueprint_core::error!("Producer stream ended unexpectedly");
                            let _ = shutdown_tx.send(true);
                            return Err(Error::JobCall("Producer stream ended".into()));
                        }
                    }
                }
                result = &mut background_services => {
                    let (result, _, remaining_background_services) = result;
                    match result {
                        Ok(()) => {
                            blueprint_core::warn!("A background service has finished running");
                        },
                        Err(e) => {
                            blueprint_core::error!("A background service failed: {:?}", e);
                            let _ = shutdown_tx.send(true);
                            return Err(e);
                        }
                    }

                    if remaining_background_services.is_empty() {
                        blueprint_core::warn!("All background services have ended");
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
