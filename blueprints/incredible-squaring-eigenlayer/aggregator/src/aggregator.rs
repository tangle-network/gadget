use alloy_network::{Ethereum, EthereumWallet, NetworkWallet};
use alloy_primitives::{keccak256, Address};
use alloy_provider::{Provider, ProviderBuilder, WsConnect};
use alloy_rpc_types::Filter;
use alloy_sol_types::{SolEvent, SolType};
use color_eyre::Result;
use eigensdk::{
    client_avsregistry::reader::AvsRegistryChainReader,
    crypto_bls::{
        convert_to_g1_point, convert_to_g2_point, BlsG1Point, BlsG2Point, OperatorId, Signature,
    },
    logging::get_test_logger,
    services_avsregistry::chaincaller::AvsRegistryServiceChainCaller,
    services_blsaggregation::bls_agg::{BlsAggregationServiceResponse, BlsAggregatorService},
    services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceInMemory,
    types::{
        avs::{TaskIndex, TaskResponseDigest},
        operator::{QuorumThresholdPercentage, QuorumThresholdPercentages},
    },
    utils::get_provider,
};
use futures_util::StreamExt;
use gadget_sdk::{debug, error, info};
use jsonrpc_core::{IoHandler, Params, Value};
use jsonrpc_http_server::{AccessControlAllowOrigin, DomainsValidation, ServerBuilder};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::IncredibleSquaringTaskManager::{
    self, G1Point, G2Point, NewTaskCreated, NonSignerStakesAndSignature, Task, TaskResponse,
};

const TASK_CHALLENGE_WINDOW_BLOCK: u32 = 100;
const BLOCK_TIME_SECONDS: u32 = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTaskResponse {
    pub task_response: TaskResponse,
    pub signature: Signature,
    pub operator_id: OperatorId,
}

pub struct Aggregator {
    port_address: String,
    task_manager_addr: Address,
    bls_aggregation_service: BlsAggregatorService<
        AvsRegistryServiceChainCaller<AvsRegistryChainReader, OperatorInfoServiceInMemory>,
    >,
    tasks: HashMap<TaskIndex, Task>,
    tasks_responses: HashMap<TaskIndex, HashMap<TaskResponseDigest, TaskResponse>>,
    http_rpc_url: String,
    wallet: EthereumWallet,
}

impl Aggregator {
    pub async fn new(
        task_manager_addr: Address,
        registry_coordinator_addr: Address,
        operator_state_retriever_addr: Address,
        http_rpc_url: String,
        ws_rpc_url: String,
        aggregator_ip_addr: String,
        wallet: EthereumWallet,
    ) -> Result<Self> {
        let avs_registry_chain_reader = AvsRegistryChainReader::new(
            get_test_logger(),
            registry_coordinator_addr,
            operator_state_retriever_addr,
            http_rpc_url.clone(),
        )
        .await?;

        let operators_info_service = OperatorInfoServiceInMemory::new(
            get_test_logger(),
            avs_registry_chain_reader.clone(),
            ws_rpc_url,
        )
        .await;

        let cancellation_token = tokio_util::sync::CancellationToken::new();
        let operators_info_clone = operators_info_service.clone();
        let token_clone = cancellation_token.clone();

        tokio::task::spawn(async move {
            operators_info_clone
                .start_service(&token_clone, 0, 200)
                .await
        });

        let avs_registry_service = AvsRegistryServiceChainCaller::new(
            avs_registry_chain_reader,
            operators_info_service.clone(),
        );

        let bls_aggregation_service = BlsAggregatorService::new(avs_registry_service);

        Ok(Self {
            port_address: aggregator_ip_addr,
            task_manager_addr,
            tasks: HashMap::new(),
            tasks_responses: HashMap::new(),
            bls_aggregation_service,
            http_rpc_url,
            wallet,
        })
    }

    pub fn start(self, ws_rpc_url: String) -> JoinHandle<()> {
        tokio::spawn(async move {
            info!("Starting aggregator");
            let aggregator = Arc::new(Mutex::new(self));

            let server_handle = tokio::spawn(Self::start_server(Arc::clone(&aggregator)));
            let tasks_handle =
                tokio::spawn(Self::process_tasks(ws_rpc_url, Arc::clone(&aggregator)));

            debug!("Server and task processing tasks have been spawned");

            tokio::select! {
                server_result = server_handle => {
                    match server_result {
                        Ok(Ok(())) => info!("Server task completed successfully"),
                        Ok(Err(e)) => error!("Server task error: {:?}", e),
                        Err(e) => error!("Server task panicked: {:?}", e),
                    }
                }
                tasks_result = tasks_handle => {
                    match tasks_result {
                        Ok(Ok(())) => info!("Task processing completed successfully"),
                        Ok(Err(e)) => error!("Task processing error: {:?}", e),
                        Err(e) => error!("Task processing panicked: {:?}", e),
                    }
                }
            }

            info!("One of the aggregator tasks has completed or errored. Aggregator will continue running.");

            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                debug!("Aggregator still running...");
            }
        })
    }

    async fn start_server(aggregator: Arc<Mutex<Self>>) -> Result<()> {
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

                    info!("Parsed signed task response: {:?}", signed_task_response);

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
        server.wait();
        Ok(())
    }

    async fn process_tasks(ws_rpc_url: String, aggregator: Arc<Mutex<Self>>) -> Result<()> {
        info!("Connecting to WebSocket RPC at: {}", ws_rpc_url);
        let provider = ProviderBuilder::new()
            .on_ws(WsConnect::new(ws_rpc_url))
            .await?;
        let filter = Filter::new().event_signature(NewTaskCreated::SIGNATURE_HASH);
        let mut stream = provider.subscribe_logs(&filter).await?.into_stream();

        while let Some(log) = stream.next().await {
            let NewTaskCreated { taskIndex, task } = log.log_decode()?.inner.data;
            println!("Received new task: {:?}", task);
            let mut aggregator = aggregator.lock().await;
            aggregator.tasks.insert(taskIndex, task.clone());

            let time_to_expiry = std::time::Duration::from_secs(
                (TASK_CHALLENGE_WINDOW_BLOCK * BLOCK_TIME_SECONDS).into(),
            );

            println!("Quorum numbers: {:?}", task.quorumNumbers.to_vec());

            match aggregator
                .bls_aggregation_service
                .initialize_new_task(
                    taskIndex,
                    task.taskCreatedBlock,
                    task.quorumNumbers.to_vec(),
                    vec![task.quorumThresholdPercentage.try_into()?; task.quorumNumbers.len()],
                    time_to_expiry,
                )
                .await
            {
                Ok(_) => debug!("Successfully initialized new task: {}", taskIndex),
                Err(e) => error!(
                    "Failed to initialize new task: {}. Error: {:?}",
                    taskIndex, e
                ),
            }
            info!("Initialized new task: {}", taskIndex);
        }

        Ok(())
    }

    async fn process_signed_task_response(
        &mut self,
        signed_task_response: SignedTaskResponse,
    ) -> Result<()> {
        info!("Processing signed task response");

        let task_index = signed_task_response.task_response.referenceTaskIndex;
        let task_response_digest = keccak256(TaskResponse::abi_encode(
            &signed_task_response.task_response,
        ));

        info!(
            "Processing signed task response for task index: {}, task response digest: {}",
            task_index, task_response_digest
        );

        if self
            .tasks_responses
            .entry(task_index)
            .or_insert_with(HashMap::new)
            .contains_key(&task_response_digest)
        {
            info!(
                "Task response digest already processed for task index: {}",
                task_index
            );
            return Ok(());
        }

        self.tasks_responses.get_mut(&task_index).unwrap().insert(
            task_response_digest,
            signed_task_response.task_response.clone(),
        );

        debug!(
            "Inserted task response for task index: {}, {:?}",
            task_index, signed_task_response
        );

        match self
            .bls_aggregation_service
            .process_new_signature(
                task_index,
                task_response_digest,
                signed_task_response.signature,
                signed_task_response.operator_id,
            )
            .await
        {
            Ok(_) => debug!(
                "Successfully processed new signature for task index: {}",
                task_index
            ),
            Err(e) => error!(
                "Failed to process new signature for task index: {}. Error: {:?}",
                task_index, e
            ),
        }

        info!(
            "Processed signed task response for task index: {}",
            task_index
        );

        if let Some(aggregated_response) = self
            .bls_aggregation_service
            .aggregated_response_receiver
            .lock()
            .await
            .recv()
            .await
        {
            info!("sending aggregated response to contract");
            self.send_aggregated_response_to_contract(aggregated_response?)
                .await?;
        }
        Ok(())
    }

    async fn send_aggregated_response_to_contract(
        &self,
        response: BlsAggregationServiceResponse,
    ) -> Result<()> {
        info!("Creating NonSignerStakesAndSignature for aggregated response");

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

        let task = &self.tasks[&response.task_index];
        let task_response =
            &self.tasks_responses[&response.task_index][&response.task_response_digest];

        let provider = get_provider(&self.http_rpc_url);
        let task_manager =
            IncredibleSquaringTaskManager::new(self.task_manager_addr, provider.clone());

        info!(
            "Sending aggregated response to contract for task index: {}",
            response.task_index
        );

        let receipt_result = task_manager
            .respondToTask(
                task.clone(),
                task_response.clone(),
                non_signer_stakes_and_signature,
            )
            .from(NetworkWallet::<Ethereum>::default_signer_address(
                &self.wallet,
            ))
            .send()
            .await;

        match receipt_result {
            Ok(receipt_future) => match receipt_future.get_receipt().await {
                Ok(receipt) => {
                    info!("Receipt obtained successfully: {:?}", receipt);
                }
                Err(e) => {
                    error!("Failed to get receipt: {:?}", e);
                    return Err(e.into());
                }
            },
            Err(e) => {
                error!("Failed to send task response: {:?}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }
}

fn check_double_mapping(
    outer_map: &HashMap<
        u32,
        HashMap<TaskResponseDigest, IncredibleSquaringTaskManager::TaskResponse>,
    >,
    outer_key: u32,
    inner_key: TaskResponseDigest,
) -> Option<&IncredibleSquaringTaskManager::TaskResponse> {
    if let Some(inner_map) = outer_map.get(&outer_key) {
        if let Some(value) = inner_map.get(&inner_key) {
            return Some(value);
        }
    }
    None
}
