//! Blueprint SDK job runners
//!
//! This crate provides the core functionality for configuring and running blueprints.
//!
//! ## Features
#![doc = document_features::document_features!()]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
// TODO: #![warn(missing_docs)]

extern crate alloc;

pub mod config;
pub mod error;

#[cfg(feature = "eigenlayer")]
pub mod eigenlayer;
#[cfg(feature = "symbiotic")]
mod symbiotic;
#[cfg(feature = "tangle")]
pub mod tangle;

use blueprint_core::error::BoxError;
use blueprint_core::{JobCall, JobResult};
use blueprint_router::Router;
use config::BlueprintEnvironment;
use core::future::{self, poll_fn};
use core::pin::Pin;
use error::RunnerError as Error;
use futures::{Future, Sink};
use futures_core::Stream;
use futures_util::{SinkExt, StreamExt, TryStreamExt, stream};
use tokio::sync::oneshot;
use tower::Service;

/// Configuration for the blueprint registration procedure
#[dynosaur::dynosaur(DynBlueprintConfig)]
pub trait BlueprintConfig: Send + Sync {
    fn register(
        &self,
        env: &BlueprintEnvironment,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        let _ = env;
        async { Ok(()) }
    }

    fn requires_registration(
        &self,
        env: &BlueprintEnvironment,
    ) -> impl Future<Output = Result<bool, Error>> + Send {
        let _ = env;
        async { Ok(true) }
    }

    /// Controls whether the runner should exit after registration
    ///
    /// Returns `true` if the runner should exit after registration, or `false` if it should continue
    fn should_exit_after_registration(&self) -> bool {
        true // By default, runners exit after registration
    }
}

unsafe impl Send for DynBlueprintConfig<'_> {}
unsafe impl Sync for DynBlueprintConfig<'_> {}

impl BlueprintConfig for () {}

#[dynosaur::dynosaur(DynBackgroundService)]
pub trait BackgroundService: Send + Sync {
    fn start(
        &self,
    ) -> impl Future<Output = Result<oneshot::Receiver<Result<(), Error>>, Error>> + Send;
}

unsafe impl Send for DynBackgroundService<'_> {}
unsafe impl Sync for DynBackgroundService<'_> {}

type Producer = Box<dyn Stream<Item = Result<JobCall, BoxError>> + Send + Sync + Unpin + 'static>;
type Consumer = Box<dyn Sink<JobResult, Error = BoxError> + Send + Sync + Unpin + 'static>;

/// A builder for a [`BlueprintRunner`]
pub struct BlueprintRunnerBuilder<F> {
    config: Box<DynBlueprintConfig<'static>>,
    env: BlueprintEnvironment,
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
    /// Set the [`Router`] for this runner
    ///
    /// A [`Router`] is the only required field in a [`BlueprintRunner`].
    #[must_use]
    pub fn router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    /// Append a [producer] to the list
    ///
    /// [producer]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/producers/index.html
    #[must_use]
    pub fn producer<E>(
        mut self,
        producer: impl Stream<Item = Result<JobCall, E>> + Send + Sync + Unpin + 'static,
    ) -> Self
    where
        E: Into<BoxError> + 'static,
    {
        self.producers.push(Box::new(producer.map_err(Into::into)));
        self
    }

    /// Append a [consumer] to the list
    ///
    /// [consumer]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/consumers/index.html
    #[must_use]
    pub fn consumer<E>(
        mut self,
        consumer: impl Sink<JobResult, Error = E> + Send + Sync + Unpin + 'static,
    ) -> Self
    where
        E: Into<BoxError> + 'static,
    {
        self.consumers
            .push(Box::new(consumer.sink_map_err(Into::into)));
        self
    }

    /// Append a background service to the list
    #[must_use]
    pub fn background_service(mut self, service: impl BackgroundService + 'static) -> Self {
        self.background_services
            .push(DynBackgroundService::boxed(service));
        self
    }

    /// Set the shutdown handler
    ///
    /// This will be run **before** the runner terminates any of the following:
    ///
    /// * [Producers]
    /// * [Consumers]
    /// * [Background Services]
    ///
    /// Meaning it is a good place to do cleanup logic, such as finalizing database transactions.
    ///
    /// [Producers]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/producers/index.html
    /// [Consumers]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/consumers/index.html
    /// [Background Services]: crate::BackgroundService
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

    /// Start the runner
    ///
    /// This will block until the runner finishes.
    ///
    /// # Errors
    ///
    /// If at any point the runner fails, an error will be returned. See [`Self::with_shutdown_handler`]
    /// to understand what this means for your running services.
    pub async fn run(self) -> Result<(), Error> {
        let Some(router) = self.router else {
            return Err(Error::NoRouter);
        };

        if self.producers.is_empty() {
            return Err(Error::NoProducers);
        }

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

/// The blueprint runner
///
/// This is responsible for orchestrating the following:
///
/// * [Producers]
/// * [Consumers]
/// * [Background Services](crate::BackgroundService)
/// * [`Router`]
///
/// # Usage
///
/// Note that this is a **full** example. All fields, with the exception of the [`Router`] can be
/// omitted.
///
/// ```no_run
/// use blueprint_router::Router;
/// use blueprint_runner::config::BlueprintEnvironment;
/// use blueprint_runner::error::RunnerError;
/// use blueprint_runner::{BackgroundService, BlueprintRunner};
/// use futures::future;
/// use tokio::sync::oneshot;
/// use tokio::sync::oneshot::Receiver;
///
/// // A dummy background service that immediately returns
/// #[derive(Clone)]
/// pub struct FooBackgroundService;
///
/// impl BackgroundService for FooBackgroundService {
///     async fn start(&self) -> Result<Receiver<Result<(), RunnerError>>, RunnerError> {
///         let (tx, rx) = oneshot::channel();
///         tokio::spawn(async move {
///             let _ = tx.send(Ok(()));
///         });
///         Ok(rx)
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     // The config is any type implementing the [BlueprintConfig] trait.
///     // In this case, () works.
///     let config = ();
///
///     // Load the blueprint environment
///     let blueprint_env = BlueprintEnvironment::default();
///
///     // Create some producer(s)
///     let some_producer = /* ... */
///     # blueprint_sdk::tangle::producer::TangleProducer::finalized_blocks(todo!()).await.unwrap();
///     # struct S;
///     # use blueprint_sdk::tangle::subxt_core::config::{PolkadotConfig, Config};
///     # impl blueprint_sdk::tangle::subxt_core::tx::signer::Signer<PolkadotConfig> for S {
///     #     fn account_id(&self) -> <PolkadotConfig as Config>::AccountId {
///     #         todo!()
///     #     }
///     #
///     #     fn address(&self) -> <PolkadotConfig as Config>::Address {
///     #         todo!()
///     #     }
///     #
///     #     fn sign(&self, signer_payload: &[u8]) -> <PolkadotConfig as Config>::Signature {
///     #         todo!()
///     #     }
///     # }
///     // Create some consumer(s)
///     let some_consumer = /* ... */
///     # blueprint_sdk::tangle::consumer::TangleConsumer::<S>::new(todo!(), todo!());
///
///     let result = BlueprintRunner::builder(config, blueprint_env)
///         .router(
///             // Add a `Router`, where each "route" is a job ID and the job function.
///             Router::new().route(0, async || "Hello, world!"),
///         )
///         // Add potentially many producers
///         .producer(some_producer)
///         // Add potentially many consumers
///         .consumer(some_consumer)
///         // Add potentially many background services
///         .background_service(FooBackgroundService)
///         // Specify what to do when an error occurs and the runner is shutting down.
///         // That can be cleanup logic, finalizing database transactions, etc.
///         .with_shutdown_handler(async { println!("Shutting down!") })
///         // Then start it up...
///         .run()
///         .await;
///
///     // The runner, after running the shutdown handler, will return
///     // an error if something goes wrong
///     if let Err(e) = result {
///         eprintln!("Runner failed! {e:?}");
///     }
/// }
/// ```
///
/// [Consumers]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/consumers/index.html
/// [Producers]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/producers/index.html
pub struct BlueprintRunner;

impl BlueprintRunner {
    /// Create a new [`BlueprintRunnerBuilder`]
    ///
    /// See the usage section of [`BlueprintRunner`]
    pub fn builder<C: BlueprintConfig + 'static>(
        config: C,
        env: BlueprintEnvironment,
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
    env: BlueprintEnvironment,
    background_services: Vec<Box<DynBackgroundService<'static>>>,
    shutdown_handler: F,
}

impl<F> FinalizedBlueprintRunner<F>
where
    F: Future<Output = ()> + Send + 'static,
{
    #[allow(trivial_casts, clippy::too_many_lines)]
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
                                    blueprint_core::trace!(
                                        count = %results.len(),
                                        "Job call(s) was processed by router"
                                    );
                                    let result_stream = stream::iter(results.into_iter().map(Ok));

                                    // Broadcast results to all consumers
                                    let send_futures = consumers.iter_mut().map(|consumer| {
                                        let mut stream_clone = result_stream.clone();
                                        async move {
                                            consumer.send_all(&mut stream_clone).await
                                        }
                                    });

                                    let result = futures::future::try_join_all(send_futures).await;
                                    blueprint_core::trace!(
                                        results = ?result.as_ref().map(|_| "success"),
                                        "Job call results were broadcasted to consumers"
                                    );
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
                () = shutdown_tx.closed() => {
                    break
                }
            }
        }

        Ok(())
    }
}
