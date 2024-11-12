use gadget_sdk::async_trait::async_trait;
use gadget_sdk::event_listener::periodic::PeriodicEventListener;
use gadget_sdk::event_listener::EventListener;
use gadget_sdk::event_utils::InitializableEventHandler;
use gadget_sdk::job;
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
        listener = PeriodicEventListener<
            2000, WebPoller, serde_json::Value, reqwest::Client
        >,
        pre_processor = pre_process,
        post_processor = post_process,
    ),
)]
// Maps a boolean value obtained from pre-processing to a u8 value
pub async fn web_poller(value: bool, client: reqwest::Client) -> Result<u8, Infallible> {
    gadget_sdk::info!("Running web_poller on value: {value}");
    Ok(value as u8)
}

// Maps a JSON response to a boolean value
pub async fn pre_process(event: serde_json::Value) -> Result<bool, gadget_sdk::Error> {
    gadget_sdk::info!("Running web_poller pre-processor on value: {event}");
    let completed = event["completed"].as_bool().unwrap_or(false);
    Ok(completed)
}

// Received the u8 value output from the job and performs any last post-processing
pub async fn post_process(job_output: u8) -> Result<(), gadget_sdk::Error> {
    gadget_sdk::info!("Running web_poller post-processor on value: {job_output}");
    if job_output == 1 {
        Ok(())
    } else {
        Err(gadget_sdk::Error::Other(
            "Job failed since query returned with a false status".to_string(),
        ))
    }
}

/// Define an event listener that polls a webserver
pub struct WebPoller {
    pub client: reqwest::Client,
}

#[async_trait]
impl EventListener<serde_json::Value, reqwest::Client> for WebPoller {
    async fn new(context: &reqwest::Client) -> Result<Self, gadget_sdk::Error>
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
