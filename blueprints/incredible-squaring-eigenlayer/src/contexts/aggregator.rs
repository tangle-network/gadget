use crate::{
    contexts::client::SignedTaskResponse,
    IncredibleSquaringTaskManager::{
        self, G1Point, G2Point, NonSignerStakesAndSignature, TaskResponse,
    },
};
use alloy_network::{Ethereum, NetworkWallet};
use alloy_primitives::keccak256;
use alloy_sol_types::SolType;
use color_eyre::Result;
use eigensdk::{
    crypto_bls::{convert_to_g1_point, convert_to_g2_point, BlsG1Point, BlsG2Point},
    services_blsaggregation::bls_agg::BlsAggregationServiceResponse,
    utils::get_provider,
};
use gadget_sdk::{
    config::StdGadgetConfiguration,
    ctx::{EigenlayerContext, KeystoreContext},
    debug, error, info,
};
use jsonrpc_core::{IoHandler, Params, Value};
use jsonrpc_http_server::{AccessControlAllowOrigin, DomainsValidation, ServerBuilder};
use std::{collections::VecDeque, net::SocketAddr, sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tokio::{
    sync::{oneshot, Mutex},
    time::interval,
};

use crate::IncredibleSquaringTaskManager::Task;
use alloy_network::EthereumWallet;
use alloy_primitives::Address;
use eigensdk::client_avsregistry::reader::AvsRegistryChainReader;
use eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller;
use eigensdk::services_blsaggregation::bls_agg::BlsAggregatorService;
use eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory;
use eigensdk::types::avs::{TaskIndex, TaskResponseDigest};
use std::collections::HashMap;

#[derive(Clone, EigenlayerContext, KeystoreContext)]
pub struct AggregatorContext {
    pub port_address: String,
    pub task_manager_address: Address,
    pub tasks: Arc<Mutex<HashMap<TaskIndex, Task>>>,
    pub tasks_responses: Arc<Mutex<HashMap<TaskIndex, HashMap<TaskResponseDigest, TaskResponse>>>>,
    pub bls_aggregation_service: Option<
        Arc<
            Mutex<
                BlsAggregatorService<
                    AvsRegistryServiceChainCaller<
                        AvsRegistryChainReader,
                        OperatorInfoServiceInMemory,
                    >,
                >,
            >,
        >,
    >,
    pub http_rpc_url: String,
    pub wallet: EthereumWallet,
    pub response_cache: Arc<Mutex<VecDeque<SignedTaskResponse>>>,
    #[config]
    pub sdk_config: StdGadgetConfiguration,
}

impl AggregatorContext {
    pub async fn new(
        port_address: String,
        task_manager_address: Address,
        http_rpc_url: String,
        wallet: EthereumWallet,
        sdk_config: StdGadgetConfiguration,
    ) -> Result<Self, std::io::Error> {
        let mut aggregator_context = AggregatorContext {
            port_address,
            task_manager_address,
            tasks: Arc::new(Mutex::new(HashMap::new())),
            tasks_responses: Arc::new(Mutex::new(HashMap::new())),
            bls_aggregation_service: None,
            http_rpc_url,
            wallet,
            response_cache: Arc::new(Mutex::new(VecDeque::new())),
            sdk_config,
        };

        // Initialize the bls registry service
        let bls_service = aggregator_context
            .bls_aggregation_service_in_memory()
            .await?;
        aggregator_context.bls_aggregation_service = Some(Arc::new(Mutex::new(bls_service)));

        Ok(aggregator_context)
    }

    pub fn start(self, _ws_rpc_url: String) -> (JoinHandle<()>, oneshot::Sender<()>) {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let aggregator = Arc::new(Mutex::new(self));

        let handle = tokio::spawn(async move {
            info!("Starting aggregator RPC server");

            let server_handle =
                tokio::spawn(Self::start_server(Arc::clone(&aggregator), shutdown_rx));
            let process_handle =
                tokio::spawn(Self::process_cached_responses(Arc::clone(&aggregator)));

            tokio::select! {
                _ = server_handle => {
                    error!("Server task unexpectedly finished");
                }
                _ = process_handle => {
                    error!("Process cached responses task unexpectedly finished");
                }
            }
        });

        (handle, shutdown_tx)
    }

    async fn start_server(
        aggregator: Arc<Mutex<Self>>,
        shutdown: oneshot::Receiver<()>,
    ) -> Result<()> {
        let mut io = IoHandler::new();
        io.add_method("process_signed_task_response", {
            let aggregator = Arc::clone(&aggregator);
            move |params: Params| {
                let aggregator = Arc::clone(&aggregator);
                async move {
                    // Parse the outer structure first
                    let outer_params: Value = params.parse()?;

                    // Extract the inner "params" object
                    let inner_params = outer_params.get("params").ok_or_else(|| {
                        jsonrpc_core::Error::invalid_params("Missing 'params' field")
                    })?;

                    // Now parse the inner params as SignedTaskResponse
                    let signed_task_response: SignedTaskResponse =
                        serde_json::from_value(inner_params.clone()).map_err(|e| {
                            jsonrpc_core::Error::invalid_params(format!(
                                "Invalid SignedTaskResponse: {}",
                                e
                            ))
                        })?;

                    aggregator
                        .lock()
                        .await
                        .process_signed_task_response(signed_task_response)
                        .await
                        .map(|_| Value::Bool(true))
                        .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))
                }
            }
        });

        let socket: SocketAddr = aggregator.lock().await.port_address.parse()?;
        let server = ServerBuilder::new(io)
            .cors(DomainsValidation::AllowOnly(vec![
                AccessControlAllowOrigin::Any,
            ]))
            .start_http(&socket)?;

        info!("Server running at {}", socket);

        // Create a close handle before we move the server
        let close_handle = server.close_handle();

        // Use tokio::select! to wait for either the server to finish or the shutdown signal
        tokio::select! {
            _ = async { server.wait() } => {
                info!("Server has stopped");
            }
            _ = shutdown => {
                info!("Initiating server shutdown");
                close_handle.close();
            }
        }
        Ok(())
    }

    async fn process_signed_task_response(&mut self, resp: SignedTaskResponse) -> Result<()> {
        let task_index = resp.task_response.referenceTaskIndex;
        let task_response_digest = keccak256(TaskResponse::abi_encode(&resp.task_response));

        info!(
            "Caching signed task response for task index: {}, task response digest: {}",
            task_index, task_response_digest
        );

        self.response_cache.lock().await.push_back(resp);

        Ok(())
    }

    async fn process_cached_responses(aggregator: Arc<Mutex<Self>>) {
        let mut interval = interval(Duration::from_secs(6));

        loop {
            interval.tick().await;

            let mut aggregator = aggregator.lock().await;
            let responses_to_process = aggregator.response_cache.lock().await.clone();

            for resp in responses_to_process {
                if let Err(e) = aggregator.process_response(resp).await {
                    error!("Failed to process cached response: {:?}", e);
                    // Continue processing other responses without failing
                } else {
                    aggregator.response_cache.lock().await.pop_front();
                }
            }
        }
    }

    async fn process_response(&mut self, resp: SignedTaskResponse) -> Result<()> {
        let SignedTaskResponse {
            task_response,
            signature,
            operator_id,
        } = resp.clone();
        let task_index = task_response.referenceTaskIndex;
        let task_response_digest = keccak256(TaskResponse::abi_encode(&task_response));

        if self
            .tasks_responses
            .lock()
            .await
            .entry(task_index)
            .or_default()
            .contains_key(&task_response_digest)
        {
            info!(
                "Task response digest already processed for task index: {}",
                task_index
            );
            return Ok(());
        }

        info!(
            "Processing signed task response for task index: {}, task response digest: {}",
            task_index, task_response_digest
        );

        self.bls_aggregation_service
            .as_ref()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "BLS Aggregation Service not initialized",
                )
            })?
            .lock()
            .await
            .process_new_signature(task_index, task_response_digest, signature, operator_id)
            .await?;

        if let Some(tasks_responses) = self.tasks_responses.lock().await.get_mut(&task_index) {
            tasks_responses.insert(task_response_digest, task_response.clone());
        }

        debug!(
            "Successfully processed new signature for task index: {}",
            task_index
        );

        if let Some(aggregated_response) = self
            .bls_aggregation_service
            .as_ref()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "BLS Aggregation Service not initialized",
                )
            })?
            .lock()
            .await
            .aggregated_response_receiver
            .lock()
            .await
            .recv()
            .await
        {
            self.send_aggregated_response_to_contract(aggregated_response?)
                .await?;
        }
        Ok(())
    }

    // async fn process_signed_task_response(&mut self, resp: SignedTaskResponse) -> Result<()> {
    //     let SignedTaskResponse {
    //         task_response,
    //         signature,
    //         operator_id,
    //     } = resp.clone();
    //     let task_index = task_response.referenceTaskIndex;
    //     let task_response_digest = keccak256(TaskResponse::abi_encode(&task_response));

    //     info!(
    //         "Processing signed task response for task index: {}, task response digest: {}",
    //         task_index, task_response_digest
    //     );

    //     if self
    //         .tasks_responses
    //         .lock()
    //         .await
    //         .entry(task_index)
    //         .or_default()
    //         .contains_key(&task_response_digest)
    //     {
    //         info!(
    //             "Task response digest already processed for task index: {}",
    //             task_index
    //         );
    //         return Ok(());
    //     }

    //     if let Some(tasks_responses) = self.tasks_responses.lock().await.get_mut(&task_index) {
    //         tasks_responses.insert(task_response_digest, task_response.clone());
    //     }

    //     debug!(
    //         "Inserted task response for task index: {}, {:?}",
    //         task_index, resp
    //     );

    //     if let Err(e) = self
    //         .bls_aggregation_service_in_memory()
    //         .await?
    //         .process_new_signature(task_index, task_response_digest, signature, operator_id)
    //         .await
    //     {
    //         error!(
    //             "Failed to process new signature for task index: {}. Error: {:?}",
    //             task_index, e
    //         );
    //     } else {
    //         debug!(
    //             "Successfully processed new signature for task index: {}",
    //             task_index
    //         );
    //     }

    //     if let Some(aggregated_response) = self
    //         .bls_aggregation_service_in_memory()
    //         .await?
    //         .aggregated_response_receiver
    //         .lock()
    //         .await
    //         .recv()
    //         .await
    //     {
    //         self.send_aggregated_response_to_contract(aggregated_response?)
    //             .await?;
    //     }
    //     Ok(())
    // }

    async fn send_aggregated_response_to_contract(
        &self,
        response: BlsAggregationServiceResponse,
    ) -> Result<()> {
        let non_signer_stakes_and_signature = NonSignerStakesAndSignature {
            nonSignerPubkeys: response
                .non_signers_pub_keys_g1
                .into_iter()
                .map(to_g1_point)
                .collect(),
            nonSignerQuorumBitmapIndices: response.non_signer_quorum_bitmap_indices,
            quorumApks: response
                .quorum_apks_g1
                .into_iter()
                .map(to_g1_point)
                .collect(),
            apkG2: to_g2_point(response.signers_apk_g2),
            sigma: to_g1_point(response.signers_agg_sig_g1.g1_point()),
            quorumApkIndices: response.quorum_apk_indices,
            totalStakeIndices: response.total_stake_indices,
            nonSignerStakeIndices: response.non_signer_stake_indices,
        };

        fn to_g1_point(pk: BlsG1Point) -> G1Point {
            let pt = convert_to_g1_point(pk.g1()).expect("Invalid G1 point");
            G1Point { X: pt.X, Y: pt.Y }
        }

        fn to_g2_point(pk: BlsG2Point) -> G2Point {
            let pt = convert_to_g2_point(pk.g2()).expect("Invalid G2 point");
            G2Point { X: pt.X, Y: pt.Y }
        }

        let tasks = self.tasks.lock().await;
        let task_responses = self.tasks_responses.lock().await;
        let task = tasks.get(&response.task_index).expect("Task not found");
        let task_response = task_responses
            .get(&response.task_index)
            .and_then(|responses| responses.get(&response.task_response_digest))
            .expect("Task response not found");

        let provider = get_provider(&self.http_rpc_url);
        let task_manager =
            IncredibleSquaringTaskManager::new(self.task_manager_address, provider.clone());

        let _ = task_manager
            .respondToTask(
                task.clone(),
                task_response.clone(),
                non_signer_stakes_and_signature,
            )
            .from(NetworkWallet::<Ethereum>::default_signer_address(
                &self.wallet,
            ))
            .send()
            .await?
            .get_receipt()
            .await?;

        info!(
            "Sent aggregated response to contract for task index: {}",
            response.task_index,
        );

        Ok(())
    }
}
