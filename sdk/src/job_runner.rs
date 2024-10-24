use crate::config::GadgetConfiguration;
use crate::event_listener::markers;
use crate::events_watcher::InitializableEventHandler;
use crate::{info, tx};
use sp_core::Pair;
use std::future::Future;
use std::pin::Pin;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets as TangleSubxtPriceTargets;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No jobs registered. Make sure to add a job with `MultiJobRunner::add_job`")]
    NoJobs,
    #[error("Job already initialized")]
    AlreadyInitialized,

    #[error(transparent)]
    Recv(#[from] tokio::sync::oneshot::error::RecvError),
}

/// Wrapper for `tangle_subxt`'s [`PriceTargets`]
///
/// This provides a [`Default`] impl for a zeroed-out [`PriceTargets`].
///
/// [`PriceTargets`]: tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets
pub struct PriceTargets(TangleSubxtPriceTargets);

impl From<TangleSubxtPriceTargets> for PriceTargets {
    fn from(t: TangleSubxtPriceTargets) -> Self {
        PriceTargets(t)
    }
}

impl Default for PriceTargets {
    fn default() -> Self {
        Self(TangleSubxtPriceTargets {
            cpu: 0,
            mem: 0,
            storage_hdd: 0,
            storage_ssd: 0,
            storage_nvme: 0,
        })
    }
}

pub trait ScopedFuture<'a>: Future + 'a {}
impl<'a, T: Future + 'a> ScopedFuture<'a> for T {}

pub(crate) type RegisterCall<'a> =
    Pin<Box<dyn ScopedFuture<'a, Output = Result<(), crate::Error>>>>;

/// A builder for blueprint jobs
///
/// Unless custom registration functions are needed, this can be avoided. See [`MultiJobRunner::job`].
pub struct JobBuilder<'a, T>
where
    T: InitializableEventHandler + Send + 'a,
{
    event_handler: T,
    price_targets: Option<PriceTargets>,
    registration: Option<RegisterCall<'a>>,
}

impl<'a, T> From<T> for JobBuilder<'a, T>
where
    T: InitializableEventHandler + Send + 'a,
{
    fn from(value: T) -> Self {
        Self {
            event_handler: value,
            price_targets: None,
            registration: None,
        }
    }
}

impl<'a, T, P> From<(T, P)> for JobBuilder<'a, T>
where
    T: InitializableEventHandler + Send + 'a,
    T: markers::IsTangle,
    P: Into<PriceTargets>,
{
    fn from(value: (T, P)) -> Self {
        Self {
            event_handler: value.0,
            price_targets: Some(value.1.into()),
            registration: None,
        }
    }
}

impl<'a, T> JobBuilder<'a, T>
where
    T: InitializableEventHandler + Send + 'a,
{
    /// Create a new `JobBuilder`
    pub fn new(event_handler: T) -> Self {
        Self {
            event_handler,
            price_targets: None,
            registration: None,
        }
    }

    /// Set the registration function
    pub fn registration<
        Fut: ScopedFuture<'a, Output = Result<(), crate::Error>> + 'a,
        Input: 'a,
    >(
        mut self,
        context: Input,
        register_call: fn(Input) -> Fut,
    ) -> Self {
        let future = register_call(context);
        self.registration = Some(Box::pin(future));
        self
    }

    /// Set the price targets
    pub fn price_targets(mut self, targets: PriceTargets) -> Self {
        self.price_targets = Some(targets);
        self
    }
}

pub type EnqueuedJobRunners<'a> = Vec<JobRunner<'a>>;

pub type JobRunner<'a> = Pin<
    Box<
        dyn ScopedFuture<
            'a,
            Output = Option<tokio::sync::oneshot::Receiver<Result<(), crate::Error>>>,
        >,
    >,
>;

pub struct MultiJobRunner<'a> {
    pub(crate) enqueued_job_runners: EnqueuedJobRunners<'a>,
    pub(crate) env: Option<GadgetConfiguration<parking_lot::RawRwLock>>,
}

impl<'a> MultiJobRunner<'a> {
    /// Create a new `MultiJobRunner`
    pub fn new<T: Into<Option<GadgetConfiguration<parking_lot::RawRwLock>>>>(env: T) -> Self {
        Self {
            enqueued_job_runners: Vec::new(),
            env: env.into(),
        }
    }

    /// Add a job to the job runner
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_sdk::job_runner::{JobBuilder, MultiJobRunner, PriceTargets};
    /// use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services;
    ///
    /// # #[gadget_sdk::main(env)]
    /// # async fn main() {
    /// #     mod blueprint {
    /// #         #[gadget_sdk::job(
    /// #              id = 0,
    /// #              params(x),
    /// #              result(_),
    /// #              event_listener(
    /// #                  listener = TangleEventListener,
    /// #                  event = JobCalled,
    /// #              ),
    /// #          )]
    /// #          pub fn xsquare(x: u64) -> Result<u64, std::convert::Infallible> {
    /// #              Ok(x.saturating_pow(2u32))
    /// #          }
    /// #         #[gadget_sdk::job(
    /// #              id = 1,
    /// #              params(x),
    /// #              result(_),
    /// #              event_listener(
    /// #                  listener = TangleEventListener,
    /// #                  event = JobCalled,
    /// #              ),
    /// #          )]
    /// #          pub fn xsquare2(x: u64) -> Result<u64, std::convert::Infallible> {
    /// #              Ok(x.saturating_pow(2u32))
    /// #          }
    /// #         #[gadget_sdk::job(
    /// #              id = 2,
    /// #              params(x),
    /// #              result(_),
    /// #              event_listener(
    /// #                  listener = TangleEventListener,
    /// #                  event = JobCalled,
    /// #              ),
    /// #          )]
    /// #          pub fn xsquare3(x: u64) -> Result<u64, std::convert::Infallible> {
    /// #              Ok(x.saturating_pow(2u32))
    /// #          }
    /// #     }
    /// let client = env.client().await?;
    /// let signer = env.first_sr25519_signer()?;
    /// let x_square = blueprint::XsquareEventHandler {
    ///     service_id: env.service_id.unwrap(),
    ///     client: client.clone(),
    ///     signer: signer.clone(),
    /// };
    ///
    /// let x_square2 = blueprint::XsquareEventHandler {
    ///     service_id: env.service_id.unwrap(),
    ///     client: client.clone(),
    ///     signer: signer.clone(),
    /// };
    ///
    /// let custom_price_targets = services::PriceTargets {
    ///     cpu: 5,
    ///     mem: 10,
    ///     storage_hdd: 15,
    ///     storage_ssd: 20,
    ///     storage_nvme: 25,
    /// };
    ///
    /// let x_square3 = blueprint::XsquareEventHandler {
    ///     service_id: env.service_id.unwrap(),
    ///     client: client.clone(),
    ///     signer: signer.clone(),
    /// };
    ///
    /// MultiJobRunner::new(env)
    ///     .job(x_square)
    ///     // With custom price targets
    ///     .job((x_square2, custom_price_targets))
    ///     // With custom registration
    ///     .job(JobBuilder::new(x_square3).registration(1, my_registration))
    ///     .run()
    ///     .await?;
    /// # Ok(()) }
    ///
    /// async fn my_registration(foo: u32) -> Result<(), gadget_sdk::Error> {
    ///     // ...
    ///     Ok(())
    /// }
    /// ```
    pub fn job<J, T>(&mut self, job: J) -> &mut Self
    where
        J: Into<JobBuilder<'a, T>>,
        T: InitializableEventHandler + Send + 'a,
    {
        let JobBuilder {
            event_handler,
            price_targets,
            mut registration,
        } = job.into();

        // Skip registration if in test mode
        let skip_registration = self.env.as_ref().map(|r| r.test_mode).unwrap_or(true);
        if skip_registration {
            registration = None;
        }

        if !skip_registration && registration.is_none() {
            let env = self
                .env
                .clone()
                .expect("Must have an env when using tangle");

            let price_targets = price_targets.unwrap_or(PriceTargets::default());

            let future = tangle_registration((env, price_targets));
            registration = Some(Box::pin(future));
        }

        let task = Box::pin(async move {
            if let Some(registration) = registration {
                if let Err(err) = registration.await {
                    crate::error!("Failed to register job: {err:?}");
                    return None;
                }
            }

            event_handler.init_event_handler().await
        });

        self.enqueued_job_runners.push(task);
        self
    }

    /// Start the job runner
    ///
    /// # Errors
    ///
    /// * No jobs are registered
    /// * A job exits prematurely
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_sdk::job_runner::MultiJobRunner;
    ///
    /// # #[gadget_sdk::main(env)]
    /// # async fn main() {
    /// #     mod blueprint {
    /// #         #[gadget_sdk::job(
    /// #              id = 0,
    /// #              params(x),
    /// #              result(_),
    /// #              event_listener(
    /// #                  listener = TangleEventListener,
    /// #                  event = JobCalled,
    /// #              ),
    /// #          )]
    /// #          pub fn xsquare(x: u64) -> Result<u64, std::convert::Infallible> {
    /// #              Ok(x.saturating_pow(2u32))
    /// #          }
    /// #     }
    /// let client = env.client().await?;
    /// let signer = env.first_sr25519_signer()?;
    /// let x_square = blueprint::XsquareEventHandler {
    ///     service_id: env.service_id.unwrap(),
    ///     client: client.clone(),
    ///     signer,
    /// };
    ///
    /// MultiJobRunner::new(env).job(x_square).run().await?;
    /// # Ok(()) }
    /// ```
    pub async fn run(&mut self) -> Result<(), crate::Error> {
        if self.enqueued_job_runners.is_empty() {
            return Err(Error::NoJobs.into());
        }

        let mut futures = Vec::new();
        let tasks = std::mem::take(&mut self.enqueued_job_runners);
        for job_runner in tasks {
            futures.push(job_runner);
        }

        let receivers = futures::future::join_all(futures).await;

        // For each receiver, take the value out of it and add it to an ordered futures list. If any values
        // is None, return an error stating that the job already initialized
        let mut ordered_futures = Vec::new();
        for receiver in receivers {
            let receiver = receiver.ok_or(Error::AlreadyInitialized)?;
            ordered_futures.push(receiver);
        }

        let (res, job_num, _) = futures::future::select_all(ordered_futures).await;
        let err = res.map_err(Error::from)?;

        crate::error!("Job {job_num} exited prematurely");
        err
    }
}

async fn tangle_registration(
    this: (GadgetConfiguration<parking_lot::RawRwLock>, PriceTargets),
) -> Result<(), crate::Error> {
    let (this, price_targets) = this;
    let client = this.client().await?;
    let signer = this.first_sr25519_signer()?;
    let ecdsa_pair = this.first_ecdsa_signer()?;

    let xt = api::tx().services().register(
        this.blueprint_id,
        services::OperatorPreferences {
            key: ecdsa_pair.signer().public().0,
            price_targets: price_targets.0,
        },
        Default::default(),
    );

    // send the tx to the tangle and exit.
    let result = tx::tangle::send(&client, &signer, &xt).await?;
    info!("Registered operator with hash: {:?}", result);
    Ok(())
}
