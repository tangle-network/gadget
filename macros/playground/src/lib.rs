#![allow(dead_code)]
use gadget_sdk::clients::tangle::runtime::TangleClient;
use gadget_sdk::event_listener::tangle::jobs::{services_post_processor, services_pre_processor};
use gadget_sdk::event_listener::tangle::TangleEventListener;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::{
    JobCalled, JobResultSubmitted,
};
use gadget_sdk::{benchmark, job, registration_hook, report, request_hook};

#[derive(Debug, Clone, Copy)]
pub enum Error {
    InvalidKeygen,
    InvalidSignature,
    InvalidRefresh,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let msg = match self {
            Error::InvalidKeygen => "Invalid Keygen",
            Error::InvalidSignature => "Invalid Signature",
            Error::InvalidRefresh => "Invalid Refresh",
        };
        write!(f, "{}", msg)
    }
}

impl std::error::Error for Error {}
#[derive(Copy, Clone)]
pub struct MyContext;

// ==================
//       Jobs
// ==================

/// Simple Threshold (t) Keygen Job for n parties.
#[job(id = 0, params(n, t), event_listener(listener = TangleEventListener<TangleClient, JobCalled>, pre_processor = services_pre_processor))]
pub fn keygen(context: TangleClient, n: u16, t: u16) -> Result<Vec<u8>, Error> {
    let _ = (n, t, context);
    Ok(vec![0; 33])
}

/// Sign a message using a key generated by the keygen job.
#[job(
    id = 1,
    params(keygen_id, data),
    event_listener(listener = TangleEventListener<TangleClient, JobCalled>, pre_processor = services_pre_processor),
)]
pub async fn sign(context: TangleClient, keygen_id: u64, data: Vec<u8>) -> Result<Vec<u8>, Error> {
    let _ = (keygen_id, data);
    Ok(vec![0; 65])
}

#[job(
    id = 2,
    params(keygen_id, new_t),
    event_listener(
        listener = TangleEventListener<TangleClient, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ),
)]
pub fn refresh(
    context: TangleClient,
    keygen_id: u64,
    new_t: Option<u8>,
) -> Result<Vec<u64>, Error> {
    let _ = (keygen_id, new_t);
    Ok(vec![0; 33])
}

/// Say hello to someone or the world.
#[job(id = 3, params(who),
    event_listener(
        listener = TangleEventListener<TangleClient, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ))]
pub fn say_hello(context: TangleClient, who: Option<String>) -> Result<String, Error> {
    match who {
        Some(who) => Ok(format!("Hello, {}!", who)),
        None => Ok("Hello, World!".to_string()),
    }
}

// ==================
//       Hooks
// ==================

#[registration_hook]
pub fn on_register(pubkey: Vec<u8>);

#[request_hook]
pub fn on_request(nft_id: u64);

// ==================
//      Reports
// ==================

/// Report function for the keygen job.
#[report(
    job_id = 0,
    params(n, t, msgs),
    event_listener(
        listener = TangleEventListener<TangleClient, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ),

    report_type = "job",
    verifier(evm = "KeygenContract")
)]
fn report_keygen(
    context: TangleClient,
    n: u16,
    t: u16,
    msgs: Vec<Vec<u8>>,
) -> Result<u32, gadget_sdk::Error> {
    let _ = (n, t, msgs);
    Ok(0)
}

#[report(
    params(uptime, response_time, error_rate),
    event_listener(listener = TangleEventListener<TangleClient, JobResultSubmitted>, pre_processor = services_pre_processor,),

    report_type = "qos",
    interval = 3600,
    metric_thresholds(uptime = 99, response_time = 1000, error_rate = 5)
)]
fn report_service_health(
    context: TangleClient,
    uptime: u64,
    response_time: u64,
    error_rate: u64,
) -> Result<Vec<u8>, gadget_sdk::Error> {
    let mut issues = Vec::new();
    if uptime < 99 {
        issues.push(b"Low uptime".to_vec());
    }
    if response_time > 1000 {
        issues.push(b"High response time".to_vec());
    }
    if error_rate > 5 {
        issues.push(b"High error rate".to_vec());
    }
    Ok(issues.concat())
}

// ==================
//   Benchmarks
// ==================
#[benchmark(job_id = 0, cores = 2)]
fn keygen_2_of_3() {
    let n = 3;
    let t = 2;
    let result = keygen(&TangleClient, n, t);
    assert!(result.is_ok());
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use gadget_sdk as sdk;
    use gadget_sdk::runners::BlueprintRunner;
    use sdk::event_listener::periodic::PeriodicEventListener;
    use sdk::event_listener::EventListener;
    use sdk::job;
    use std::convert::Infallible;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn generated_blueprint() {
        eprintln!("{}", super::KEYGEN_JOB_DEF);
        assert_eq!(super::KEYGEN_JOB_ID, 0);
        eprintln!("{}", super::REGISTRATION_HOOK_PARAMS);
    }

    #[test]
    fn sdk_main() {
        setup_env();

        #[sdk::main]
        async fn main() {
            Ok(())
        }
    }

    #[test]
    fn sdk_main_with_env() {
        setup_env();

        #[sdk::main(env)]
        async fn main() {
            Ok(())
        }
    }

    #[test]
    fn sdk_main_with_tokio_params_1() {
        setup_env();

        #[sdk::main(env, flavor = "multi_thread")]
        async fn main() {
            Ok(())
        }
    }

    #[test]
    fn sdk_main_with_tokio_params_2() {
        setup_env();

        #[sdk::main(env, flavor = "multi_thread", worker_threads = 4)]
        async fn main() {
            Ok(())
        }
    }

    #[test]
    fn sdk_main_with_tokio_params_mixed_order() {
        setup_env();

        #[sdk::main(flavor = "multi_thread", env, worker_threads = 4)]
        async fn main() {
            Ok(())
        }
    }

    #[job(
        id = 0,
        params(value),

        event_listener(
            listener = PeriodicEventListener<1500, WebPoller, serde_json::Value, Arc<AtomicUsize>>,
            pre_processor = pre_process,
            post_processor = post_process,
        ),
    )]
    // Maps a boolean value obtained from pre-processing to a u8 value
    pub async fn web_poller(value: bool, count: Arc<AtomicUsize>) -> Result<u8, Infallible> {
        gadget_sdk::info!("Running web_poller on value: {value}");
        Ok(value as u8)
    }

    async fn pre_process(event: serde_json::Value) -> Result<bool, gadget_sdk::Error> {
        gadget_sdk::info!("Running web_poller pre-processor on value: {event}");
        let completed = event["completed"].as_bool().unwrap_or(false);
        Ok(completed)
    }

    async fn post_process(job_output: u8) -> Result<(), gadget_sdk::Error> {
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
        pub count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl EventListener<serde_json::Value, Arc<AtomicUsize>> for WebPoller {
        async fn new(context: &Arc<AtomicUsize>) -> Result<Self, gadget_sdk::Error>
        where
            Self: Sized,
        {
            let client = reqwest::Client::new();
            Ok(Self {
                client,
                count: context.clone(),
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
                self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Some(resp)
            } else {
                None
            }
        }

        /// Implement any handler logic when an event is received
        async fn handle_event(
            &mut self,
            _event: serde_json::Value,
        ) -> Result<(), gadget_sdk::Error> {
            unreachable!("Not called here")
        }
    }

    #[tokio::test]
    async fn test_web_poller_event_workflow_works() {
        gadget_sdk::logging::setup_log();
        let env = gadget_sdk::config::load(Default::default()).expect("Failed to load environment");
        let count = &Arc::new(AtomicUsize::new(0));
        let job = WebPollerEventHandler {
            count: count.clone(),
        };

        let task0 = async move {
            BlueprintRunner::new((), env)
                .job(job)
                .run()
                .await
                .expect("Job failed");
        };

        let periodic_poller = async move {
            loop {
                if count.load(std::sync::atomic::Ordering::SeqCst) > 3 {
                    break;
                }

                tokio::time::sleep(Duration::from_millis(500)).await
            }
        };

        tokio::select! {
            _ = task0 => {
                panic!("Should not occur")
            },
            _ = periodic_poller => {
                assert!(count.load(std::sync::atomic::Ordering::SeqCst) > 3);
            },
        }
    }

    fn setup_env() {
        // Add required env vars for all child processes/gadgets
        let env_vars = [
            ("RPC_URL".to_string(), "ws://127.0.0.1".to_string()),
            ("KEYSTORE_URI".to_string(), "/".to_string()),
            ("BLUEPRINT_ID".to_string(), 1.to_string()),
            ("SERVICE_ID".to_string(), 1.to_string()),
            ("DATA_DIR".to_string(), "/".to_string()),
        ];

        for (key, value) in env_vars {
            std::env::set_var(key, value);
        }
    }
}
