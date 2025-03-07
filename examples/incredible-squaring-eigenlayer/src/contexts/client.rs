use alloy_rpc_client::ReqwestClient;
use color_eyre::Result;
use eigensdk::crypto_bls::{OperatorId, Signature};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{sleep, Duration};
use tracing::{debug, info};

use crate::IIncredibleSquaringTaskManager::TaskResponse;

const MAX_RETRIES: u32 = 5;
const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTaskResponse {
    pub task_response: TaskResponse,
    pub signature: Signature,
    pub operator_id: OperatorId,
}

/// Client for interacting with the Aggregator RPC server
#[derive(Debug, Clone)]
pub struct AggregatorClient {
    client: ReqwestClient,
}

impl AggregatorClient {
    /// Creates a new AggregatorClient
    pub fn new(aggregator_address: &str) -> Result<Self> {
        let url = Url::parse(&format!("http://{}", aggregator_address))?;
        let client = ReqwestClient::new_http(url);
        Ok(Self { client })
    }

    /// Sends a signed task response to the aggregator
    pub async fn send_signed_task_response(&self, response: SignedTaskResponse) -> Result<()> {
        let params = json!({
            "params": response,
            "id": 1,
            "jsonrpc": "2.0"
        });

        for attempt in 1..=MAX_RETRIES {
            match self
                .client
                .request::<_, bool>("process_signed_task_response", &params)
                .await
            {
                Ok(true) => {
                    info!("Task response accepted by aggregator");
                    // MARK: Uncomment when metrics are implemented
                    // incredible_metrics::inc_num_tasks_accepted_by_aggregator();
                    return Ok(());
                }
                Ok(false) => debug!("Task response not accepted, retrying..."),
                Err(e) => debug!("Error sending task response: {}", e),
            }

            if attempt < MAX_RETRIES {
                let delay = INITIAL_RETRY_DELAY * 2u32.pow(attempt - 1);
                info!("Retrying in {} seconds...", delay.as_secs());
                sleep(delay).await;
            }
        }

        debug!(
            "Failed to send signed task response after {} attempts",
            MAX_RETRIES
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_client() {
        let client = AggregatorClient::new("127.0.0.1:8545");
        assert!(client.is_ok());
    }
}
