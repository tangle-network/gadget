use crate::config::BlueprintConfig;
use crate::error::RunnerError as Error;
use crate::jobs::JobBuilder;
use core::pin::Pin;

use futures::Future;
use gadget_config::GadgetConfiguration;
use gadget_event_listeners::core::InitializableEventHandler;
use tokio::sync::oneshot;

#[async_trait::async_trait]
pub trait BackgroundService: Send + Sync + 'static {
    async fn start(&self) -> Result<oneshot::Receiver<Result<(), Error>>, Error>;
}

pub struct BlueprintRunner {
    pub config: Box<dyn BlueprintConfig>,
    pub jobs: Vec<Box<dyn InitializableEventHandler + Send + 'static>>,
    pub env: GadgetConfiguration,
    pub background_services: Vec<Box<dyn BackgroundService>>,
}

impl BlueprintRunner {
    pub fn new<C: BlueprintConfig + 'static>(config: C, env: GadgetConfiguration) -> Self {
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

    pub async fn run(&mut self) -> Result<(), Error> {
        if self.config.requires_registration(&self.env).await? {
            self.config.register(&self.env).await?;
            if self.config.should_pre_register() {
                // Return from pre-registration
                return Ok(());
            }
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
                    Some(receiver) => receiver
                        .await
                        .map(|_| ())
                        .map_err(|e| Error::Recv(e.to_string())),
                    None => Ok(()),
                }
            })
                as Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>);
        }

        // Handle background services
        for receiver in background_receivers {
            all_futures.push(Box::pin(async move {
                receiver
                    .await
                    .map_err(|e| Error::Recv(e.to_string()))
                    .and(Ok(()))
            })
                as Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>);
        }

        while !all_futures.is_empty() {
            let (result, _index, remaining) = futures::future::select_all(all_futures).await;
            if let Err(e) = result {
                gadget_logging::error!("Job or background service failed: {:?}", e);
            }

            all_futures = remaining;
        }

        Ok(())
    }
}
