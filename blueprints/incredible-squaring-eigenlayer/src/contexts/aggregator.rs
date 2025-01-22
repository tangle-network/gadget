use crate::IBLSSignatureChecker::NonSignerStakesAndSignature;
use crate::IIncredibleSquaringTaskManager::Task;
use crate::IIncredibleSquaringTaskManager::TaskResponse;
use crate::BN254::G1Point;
use crate::BN254::G2Point;
use crate::{contexts::client::SignedTaskResponse, Error, IncredibleSquaringTaskManager};
use alloy_network::{Ethereum, NetworkWallet};
use alloy_primitives::{keccak256, Address};
use alloy_sol_types::SolType;
use jsonrpc_core::{IoHandler, Params, Value};
use jsonrpc_http_server::{AccessControlAllowOrigin, DomainsValidation, ServerBuilder};
use std::{collections::VecDeque, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::{oneshot, Mutex, Notify};
use tokio::task::JoinHandle;
use tokio::time::interval;

use alloy_network::EthereumWallet;
use blueprint_sdk::config::GadgetConfiguration;
use blueprint_sdk::contexts::eigenlayer::EigenlayerContext;
use blueprint_sdk::logging::{debug, error, info};
use blueprint_sdk::macros::contexts::{EigenlayerContext, KeystoreContext};
use blueprint_sdk::runners::core::error::RunnerError;
use blueprint_sdk::runners::core::runner::BackgroundService;
use eigensdk::client_avsregistry::reader::AvsRegistryChainReader;
use eigensdk::common::get_provider;
use eigensdk::crypto_bls::{convert_to_g1_point, convert_to_g2_point, BlsG1Point, BlsG2Point};
use eigensdk::services_avsregistry::chaincaller::AvsRegistryServiceChainCaller;
use eigensdk::services_blsaggregation::{
    bls_agg::BlsAggregatorService, bls_aggregation_service_response::BlsAggregationServiceResponse,
};
use eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory;
use eigensdk::types::avs::{TaskIndex, TaskResponseDigest};
use std::collections::HashMap;

pub type BlsAggServiceInMemory = BlsAggregatorService<
    AvsRegistryServiceChainCaller<AvsRegistryChainReader, OperatorInfoServiceInMemory>,
>;

#[derive(Clone, EigenlayerContext, KeystoreContext)]
pub struct AggregatorContext {
    pub port_address: String,
    pub task_manager_address: Address,
    pub tasks: Arc<Mutex<HashMap<TaskIndex, Task>>>,
    pub tasks_responses: Arc<Mutex<HashMap<TaskIndex, HashMap<TaskResponseDigest, TaskResponse>>>>,
    pub bls_aggregation_service: Option<Arc<Mutex<BlsAggServiceInMemory>>>,
    pub http_rpc_url: String,
    pub wallet: EthereumWallet,
    pub response_cache: Arc<Mutex<VecDeque<SignedTaskResponse>>>,
    #[config]
    pub sdk_config: GadgetConfiguration,
    shutdown: Arc<(Notify, Mutex<bool>)>,
}

impl AggregatorContext {
    pub async fn new(
        port_address: String,
        task_manager_address: Address,
        wallet: EthereumWallet,
        sdk_config: GadgetConfiguration,
    ) -> Result<Self, Error> {
        let mut aggregator_context = AggregatorContext {
            port_address,
            task_manager_address,
            tasks: Arc::new(Mutex::new(HashMap::new())),
            tasks_responses: Arc::new(Mutex::new(HashMap::new())),
            bls_aggregation_service: None,
            http_rpc_url: sdk_config.http_rpc_endpoint.clone(),
            wallet,
            response_cache: Arc::new(Mutex::new(VecDeque::new())),
            sdk_config,
            shutdown: Arc::new((Notify::new(), Mutex::new(false))),
        };

        // Initialize the bls registry service
        let bls_service = aggregator_context
            .eigenlayer_client()
            .await
            .map_err(|e| Error::Context(e.to_string()))?
            .bls_aggregation_service_in_memory()
            .await
            .map_err(|e| Error::Context(e.to_string()))?;
        aggregator_context.bls_aggregation_service = Some(Arc::new(Mutex::new(bls_service)));

        Ok(aggregator_context)
    }

    pub async fn start(self) -> JoinHandle<()> {
        let aggregator = Arc::new(Mutex::new(self));

        tokio::spawn(async move {
            info!("Starting aggregator RPC server");

            let server_handle = tokio::spawn(Self::start_server(Arc::clone(&aggregator)));

            let process_handle =
                tokio::spawn(Self::process_cached_responses(Arc::clone(&aggregator)));

            // Wait for both tasks to complete
            let (server_result, process_result) = tokio::join!(server_handle, process_handle);

            if let Err(e) = server_result {
                error!("Server task failed: {}", e);
            }
            if let Err(e) = process_result {
                error!("Process cached responses task failed: {}", e);
            }

            info!("Aggregator shutdown complete");
        })
    }

    pub async fn shutdown(&self) {
        info!("Initiating aggregator shutdown");

        // Set internal shutdown flag
        let (notify, is_shutdown) = &*self.shutdown;
        *is_shutdown.lock().await = true;
        notify.notify_waiters();
    }

    async fn start_server(aggregator: Arc<Mutex<Self>>) -> Result<(), Error> {
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

        let socket: SocketAddr = aggregator
            .lock()
            .await
            .port_address
            .parse()
            .map_err(Error::Parse)?;
        let server = ServerBuilder::new(io)
            .cors(DomainsValidation::AllowOnly(vec![
                AccessControlAllowOrigin::Any,
            ]))
            .start_http(&socket)
            .map_err(|e| Error::Context(e.to_string()))?;

        info!("Server running at {}", socket);

        // Create a close handle before we move the server
        let close_handle = server.close_handle();

        // Get shutdown components
        let shutdown = {
            let agg = aggregator.lock().await;
            agg.shutdown.clone()
        };

        // Create a channel to coordinate shutdown
        let (server_tx, server_rx) = oneshot::channel();

        // Spawn the server in a blocking task
        let server_handle = tokio::task::spawn_blocking(move || {
            server.wait();
            let _ = server_tx.send(());
        });

        // Use tokio::select! to wait for either the server to finish or the shutdown signal
        tokio::select! {
            result = server_handle => {
                info!("Server has stopped naturally");
                result.map_err(|e| {
                    error!("Server task failed: {}", e);
                    Error::Runtime(e.to_string())
                })?;
            }
            _ = async {
                let (_, is_shutdown) = &*shutdown;
                loop {
                    if *is_shutdown.lock().await {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            } => {
                info!("Initiating server shutdown");
                // Spawn a blocking task to handle server shutdown
                tokio::task::spawn_blocking(move || {
                    close_handle.close();
                }).await.map_err(|e| Error::Runtime(e.to_string()))?;

                // Wait for server to complete
                let _ = server_rx.await;
                info!("Server has stopped after shutdown");
            }
        }

        info!("Server shutdown complete");
        Ok(())
    }

    async fn process_signed_task_response(
        &mut self,
        resp: SignedTaskResponse,
    ) -> Result<(), Error> {
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

        // Get shutdown components
        let shutdown = {
            let agg = aggregator.lock().await;
            agg.shutdown.clone()
        };

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Check shutdown status first
                    if *shutdown.1.lock().await {
                        info!("Process cached responses received shutdown signal");
                        break;
                    }

                    // Get responses to process while holding the lock briefly
                    let responses_to_process = {
                        let guard = aggregator.lock().await;
                        let cache = guard.response_cache.lock().await;
                        cache.clone()
                    };

                    // Process each response without holding the main lock
                    for resp in responses_to_process {
                        let res = {
                            let mut guard = aggregator.lock().await;
                            guard.process_response(resp.clone()).await
                        };
                        match res {
                            Ok(_) => {
                                // Only remove from cache if processing succeeded
                                let guard = aggregator.lock().await;
                                let mut cache = guard.response_cache.lock().await;
                                cache.pop_front();
                            }
                            Err(e) => {
                                error!("Failed to process cached response: {:?}", e);
                                // Continue processing other responses without failing
                            }
                        }
                    }
                }
                _ = shutdown.0.notified() => {
                    if *shutdown.1.lock().await {
                        info!("Process cached responses received shutdown signal");
                        break;
                    }
                }
            }
        }
    }

    async fn process_response(&mut self, resp: SignedTaskResponse) -> Result<(), Error> {
        let SignedTaskResponse {
            task_response,
            signature,
            operator_id,
        } = resp.clone();
        let task_index = task_response.referenceTaskIndex;
        let task_response_digest = keccak256(TaskResponse::abi_encode(&task_response));

        // Check if we have the task initialized first
        if !self.tasks.lock().await.contains_key(&task_index) {
            info!(
                "Task {} not yet initialized, caching response for later processing",
                task_index
            );
            self.response_cache.lock().await.push_back(resp);
            return Ok(());
        }

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
            })
            .map_err(|e| Error::Context(e.to_string()))?
            .lock()
            .await
            .process_new_signature(task_index, task_response_digest, signature, operator_id)
            .await
            .map_err(|e| Error::Context(e.to_string()))?;

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
            })
            .map_err(|e| Error::Context(e.to_string()))?
            .lock()
            .await
            .aggregated_response_receiver
            .lock()
            .await
            .recv()
            .await
        {
            let response = aggregated_response.map_err(|e| Error::Context(e.to_string()))?;
            self.send_aggregated_response_to_contract(response).await?;
        }
        Ok(())
    }

    async fn send_aggregated_response_to_contract(
        &self,
        response: BlsAggregationServiceResponse,
    ) -> Result<(), Error> {
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
            .await
            .map_err(|e| Error::Chain(e.to_string()))?
            .get_receipt()
            .await
            .map_err(|e| Error::Chain(e.to_string()))?;

        info!(
            "Sent aggregated response to contract for task index: {}",
            response.task_index,
        );

        Ok(())
    }
}

#[async_trait::async_trait]
impl BackgroundService for AggregatorContext {
    async fn start(&self) -> Result<oneshot::Receiver<Result<(), RunnerError>>, RunnerError> {
        let handle = self.clone().start().await;
        info!("Aggregator task started");
        let (result_tx, result_rx) = oneshot::channel();

        tokio::spawn(async move {
            match handle.await {
                Ok(_) => {
                    info!("Aggregator task finished");
                    let _ = result_tx.send(Ok(()));
                }
                Err(e) => {
                    error!("Aggregator task failed: {}", e);
                    let _ = result_tx.send(Err(RunnerError::Eigenlayer(format!(
                        "Aggregator task failed: {:?}",
                        e
                    ))));
                }
            }
        });

        Ok(result_rx)
    }
}
