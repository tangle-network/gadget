use async_trait::async_trait;
use blueprint_sdk::event_listeners::core::{EventListener, InitializableEventHandler};
use blueprint_sdk::event_listeners::cronjob::{CronJob, CronJobDefinition};
use blueprint_sdk::logging::info;
use blueprint_sdk::{job, Error};
use std::convert::Infallible;

pub fn constructor() -> impl InitializableEventHandler {
    WebPollerEventHandler {
        client: reqwest::Client::new(),
    }
}

#[job(
    id = 0,
    params(value),
    event_listener(
        listener = CronJob<WebPollerContext>,
        pre_processor = pre_process,
        post_processor = post_process,
    ),
)]
// Maps a boolean value obtained from pre-processing to a u8 value
pub async fn web_poller(value: bool, client: reqwest::Client) -> Result<u8, Infallible> {
    info!("Running web_poller on value: {value}");
    Ok(value as u8)
}

// Maps a JSON response to a boolean value
pub async fn pre_process(event: serde_json::Value) -> Result<bool, Error> {
    info!("Running web_poller pre-processor on value: {event}");
    let completed = event["completed"].as_bool().unwrap_or(false);
    Ok(completed)
}

// Received the u8 value output from the job and performs any last post-processing
pub async fn post_process(job_output: u8) -> Result<(), Error> {
    info!("Running web_poller post-processor on value: {job_output}");
    if job_output == 1 {
        Ok(())
    } else {
        Err(Error::Other(
            "Job failed since query returned with a false status".to_string(),
        ))
    }
}

struct WebPollerContext(&'static str);

impl CronJobDefinition for WebPollerContext {
    fn cron(&self) -> impl Into<String> {
        self.0
    }
}

/// Define an event listener that polls a webserver
pub struct WebPoller {
    pub client: reqwest::Client,
}

#[async_trait]
impl EventListener<serde_json::Value, reqwest::Client> for WebPoller {
    type ProcessorError = Error;

    async fn new(context: &reqwest::Client) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Ok(Self {
            client: context.clone(),
        })
    }

    /// Implement the logic that polls the web server
    async fn next_event(&mut self) -> Option<serde_json::Value> {
        // Send a GET request to the JSONPlaceholder API
        let response = self
            .client
            .get("https://jsonplaceholder.typicode.com/todos/10")
            .send()
            .await
            .ok()?;

        // Check if the request was successful
        if response.status().is_success() {
            // Parse the JSON response
            let resp: serde_json::Value = response.json().await.ok()?;
            Some(resp)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestContext(&'static str);

    impl CronJobDefinition for TestContext {
        fn cron(&self) -> impl Into<String> {
            self.0
        }
    }

    #[tokio::test]
    async fn cronjob_event_listener() {
        // Run every second
        let cron_job = TestContext("1/2 * * * * *");
        let mut cronjob = CronJob::new(&cron_job).await.unwrap();
        let next_event = cronjob.next_event().await;
        assert!(next_event.is_some());
    }
}
