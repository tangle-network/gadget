use crate::config::GadgetConfiguration;
use crate::error;
use crate::event_utils::InitializableEventHandler;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::oneshot;

pub mod eigenlayer;
pub mod symbiotic;

pub mod jobs;
pub mod tangle;
pub use jobs::*;

#[derive(thiserror::Error, Debug)]
pub enum RunnerError {
    #[error("No jobs registered. Make sure to add a job with `BlueprintRunner::add_job`")]
    NoJobs,
    #[error("Job already initialized")]
    AlreadyInitialized,

    #[error(transparent)]
    Recv(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("Environment not set")]
    EnvNotSet,

    #[error("Receiver error")]
    ReceiverError,

    #[error(transparent)]
    ConfigError(#[from] crate::config::Error),

    #[error(transparent)]
    SubxtError(#[from] subxt::Error),

    #[error(transparent)]
    KeystoreError(#[from] crate::keystore::Error),

    #[error(transparent)]
    ContractError(#[from] alloy_contract::Error),

    #[error(transparent)]
    ElContractsError(#[from] eigensdk::client_elcontracts::error::ElContractsError),

    #[error(transparent)]
    AvsRegistryError(#[from] eigensdk::client_avsregistry::error::AvsRegistryError),

    #[error(transparent)]
    TransportError(#[from] alloy_transport::RpcError<alloy_transport::TransportErrorKind>),

    #[error("Environment not set")]
    EnvironmentNotSet,

    #[error("Eigenlayer error: {0}")]
    EigenlayerError(String),

    #[error("Symbiotic error: {0}")]
    SymbioticError(String),

    #[error("Invalid protocol: {0}")]
    InvalidProtocol(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

#[async_trait::async_trait]
pub trait BlueprintConfig: Send + Sync + 'static {
    async fn register(
        &self,
        _env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<(), RunnerError> {
        Ok(())
    }
    async fn requires_registration(
        &self,
        _env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<bool, RunnerError> {
        Ok(true)
    }
}

impl BlueprintConfig for () {}

#[async_trait::async_trait]
pub trait BackgroundService: Send + Sync + 'static {
    async fn start(&self) -> Result<oneshot::Receiver<Result<(), RunnerError>>, RunnerError>;
}

pub struct BlueprintRunner {
    pub(crate) config: Box<dyn BlueprintConfig>,
    pub(crate) jobs: Vec<Box<dyn InitializableEventHandler + Send + 'static>>,
    pub(crate) env: GadgetConfiguration<parking_lot::RawRwLock>,
    pub(crate) background_services: Vec<Box<dyn BackgroundService>>,
}

impl BlueprintRunner {
    pub fn new<C: BlueprintConfig + 'static>(
        config: C,
        env: GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Self {
        Self {
            config: Box::new(config),
            jobs: Vec::new(),
            background_services: Vec::new(),
            env,
        }
    }

    pub fn job<J, T>(&mut self, job: J) -> &mut Self
    where
        J: Into<JobBuilder<T>>,
        T: InitializableEventHandler + Send + 'static,
    {
        let JobBuilder { event_handler } = job.into();
        self.jobs.push(Box::new(event_handler));
        self
    }

    pub fn background_service(&mut self, service: Box<dyn BackgroundService>) -> &mut Self {
        self.background_services.push(service);
        self
    }

    pub async fn run(&mut self) -> Result<(), RunnerError> {
        if self.config.requires_registration(&self.env).await? {
            self.config.register(&self.env).await?
        }

        let mut background_receivers = Vec::new();
        for service in &self.background_services {
            let receiver = service.start().await?;
            background_receivers.push(receiver);
        }

        let mut all_futures = Vec::new();

        // Handle job futures
        for job in self.jobs.drain(..) {
            all_futures.push(Box::pin(async move {
                match job.init_event_handler().await {
                    Some(receiver) => receiver.await.map_err(RunnerError::Recv)?,
                    None => Ok(()),
                }
            })
                as Pin<Box<dyn Future<Output = Result<(), crate::Error>> + Send>>);
        }

        // Handle background services
        for receiver in background_receivers {
            all_futures.push(Box::pin(async move {
                receiver
                    .await
                    .map_err(|e| crate::Error::Runner(RunnerError::Recv(e)))
                    .and(Ok(()))
            })
                as Pin<Box<dyn Future<Output = Result<(), crate::Error>> + Send>>);
        }

        while !all_futures.is_empty() {
            let (result, _index, remaining) = futures::future::select_all(all_futures).await;
            match result {
                Ok(_) => {
                    // Job or background service completed successfully
                }
                Err(e) => {
                    eprintln!("Job or background service failed: {:?}", e);
                }
            }
            all_futures = remaining;
        }

        Ok(())
    }
}
