use blueprint_sdk::alloy::transports::http::reqwest;
use blueprint_sdk::event_listeners::core::InitializableEventHandler;
use blueprint_sdk::event_listeners::cronjob::{
    CronJob, CronJobDefinition, error::Error as CronJobError,
};
use blueprint_sdk::logging::info;
use blueprint_sdk::{Error, job};

type ProcessorError = blueprint_sdk::event_listeners::core::Error<CronJobError>;

pub fn constructor(cron: &'static str) -> impl InitializableEventHandler {
    WebPollerEventHandler {
        context: WebPollerContext::new(cron, reqwest::Client::new()),
    }
}

#[derive(Clone)]
pub struct WebPollerContext {
    cron: &'static str,
    client: reqwest::Client,
}

impl WebPollerContext {
    pub fn new(cron: &'static str, client: reqwest::Client) -> Self {
        Self { cron, client }
    }
}

impl CronJobDefinition for WebPollerContext {
    fn cron(&self) -> impl Into<String> {
        self.cron
    }
}

#[job(
    id = 1,
    event_listener(
        listener = CronJob<WebPollerContext>,
        post_processor = post_process,
    ),
)]
pub async fn web_poller(context: WebPollerContext) -> Result<u8, Error> {
    // Send a GET request to the JSONPlaceholder API
    let response = context
        .client
        .get("https://jsonplaceholder.typicode.com/todos/10")
        .send()
        .await
        .map_err(|err| Error::Other(err.to_string()))?;

    // Check if the request was successful
    if !response.status().is_success() {
        return Err(Error::Other("Request failed".to_string()));
    }

    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|err| Error::Other(err.to_string()))?;
    let completed = value["completed"].as_bool().unwrap_or(false);

    info!("Running web_poller on value: {completed}");
    Ok(completed as u8)
}

// Received the u8 value output from the job and performs any last post-processing
pub async fn post_process(job_output: u8) -> Result<(), ProcessorError> {
    info!("Running web_poller post-processor on value: {job_output}");
    if job_output == 1 {
        let result = std::env::var("WEB_POLLER_RESULT").unwrap_or("0".to_string());
        let result = result.parse::<u64>().unwrap_or(0);
        let result = result + 1;
        std::env::set_var("WEB_POLLER_RESULT", result.to_string());
        Ok(())
    } else {
        Err(ProcessorError::EventHandler(
            "Job failed since query returned with a false status".to_string(),
        ))
    }
}
