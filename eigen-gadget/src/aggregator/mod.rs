use std::{collections::HashMap, sync::Arc};

use alloy_primitives::U256;
use alloy_provider::{network::Ethereum, Provider};
use alloy_transport::Transport;
use eigen_utils::{
    avs_registry::reader::AvsRegistryChainReader,
    services::{
        avs_registry::{chain_caller::AvsRegistryServiceChainCaller, AvsRegistryServiceTrait},
        bls_aggregation::{
            BlsAggregationService, BlsAggregationServiceResponse, BlsAggregatorService,
        },
    },
    types::TaskResponse,
};
use tokio::sync::{mpsc, RwLock};

use crate::{
    avs::{
        reader::AvsReader,
        writer::AvsWriter,
        IncredibleSquaringTaskManager::{self, Task},
        SetupConfig,
    },
    operator::OperatorError,
};

pub struct Aggregator<T, P, R>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
    R: AvsRegistryServiceTrait,
{
    server_ip_port_addr: String,
    avs_writer: AvsWriter<T, P>,
    bls_aggregation_service: BlsAggregatorService<R>,
    tasks: Arc<RwLock<HashMap<u32, Task>>>,
    task_responses: Arc<RwLock<HashMap<u32, HashMap<U256, TaskResponse>>>>,
}

impl<T, P, R> Aggregator<T, P, R>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
    R: AvsRegistryServiceTrait,
{
    pub async fn new(config: &SetupConfig<T, P>) -> Result<Self, OperatorError> {
        let avs_writer = AvsWriter::from_config(config).await?;
        let avs_reader = AvsRegistryChainReader::build(
            config.registry_coordinator_addr,
            config.operator_state_retriever_addr,
            config.eth_client,
        )
        .await;
        let avs_chain_caller =
            AvsRegistryServiceChainCaller::new(operator_info_service, avs_reader);
        let (tx, rx) = mpsc::channel(100);
        let bls_aggregation_service = BlsAggregatorService::new(tx);

        Ok(Self {
            server_ip_port_addr: config.aggregator_server_ip_port_addr.clone(),
            avs_writer,
            bls_aggregation_service,
            tasks: Arc::new(RwLock::new(HashMap::new())),
            task_responses: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn start(
        self: Arc<Self>,
        ctx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<(), OperatorError> {
        let mut interval = interval(Duration::from_secs(10));
        let mut task_num = 0;

        self.logger.info("Starting aggregator.");
        self.logger.info("Starting aggregator rpc server.");
        tokio::spawn(self.clone().start_server(ctx.clone()));

        // Send the first task
        self.send_new_task(U256::from(task_num)).await?;
        task_num += 1;

        loop {
            tokio::select! {
                _ = ctx.recv() => {
                    return Ok(());
                }
                bls_agg_service_resp = self.bls_aggregation_service.get_response_channel().recv() => {
                    match bls_agg_service_resp {
                        Some(resp) => {
                            self.send_aggregated_response_to_contract(resp).await;
                        }
                        None => break,
                    }
                }
                _ = interval.tick() => {
                    if let Err(err) = self.send_new_task(U256::from(task_num)).await {
                        self.logger.error("Error sending new task", &err.to_string());
                    }
                    task_num += 1;
                }
            }
        }

        Ok(())
    }

    async fn send_new_task(&self, num_to_square: U256) -> Result<(), OperatorError> {
        self.logger
            .info("Aggregator sending new task", &num_to_square.to_string());
        let (new_task, task_index) = self
            .avs_writer
            .send_new_task_number_to_square(
                num_to_square,
                QUORUM_THRESHOLD_NUMERATOR,
                QUORUM_NUMBERS.to_vec(),
            )
            .await?;

        {
            let mut tasks = self.tasks.write().unwrap();
            tasks.insert(task_index, new_task.clone());
        }

        let quorum_threshold_percentages = vec![QUORUM_THRESHOLD_PERCENTAGE; QUORUM_NUMBERS.len()];
        let time_to_expiry = Duration::from_secs(TASK_CHALLENGE_WINDOW_BLOCK * BLOCK_TIME_SECONDS);
        self.bls_aggregation_service
            .initialize_new_task(
                task_index,
                new_task.task_created_block,
                QUORUM_NUMBERS.to_vec(),
                quorum_threshold_percentages,
                time_to_expiry,
            )
            .await;

        Ok(())
    }

    async fn send_aggregated_response_to_contract(
        &self,
        bls_agg_service_resp: BlsAggregationServiceResponse,
    ) {
        if let Some(err) = bls_agg_service_resp.err {
            self.logger
                .error("BlsAggregationServiceResponse contains an error", &err);
            return;
        }

        let non_signer_pubkeys = bls_agg_service_resp
            .non_signers_pubkeys_g1
            .into_iter()
            .map(|pubkey| IncredibleSquaringTaskManager::G1Point {
                X: pubkey.x,
                Y: pubkey.y,
            })
            .collect();

        let quorum_apks = bls_agg_service_resp
            .quorum_apks_g1
            .into_iter()
            .map(|apk| IncredibleSquaringTaskManager::G1Point { X: apk.x, Y: apk.y })
            .collect();

        let non_signer_stakes_and_signature =
            IncredibleSquaringTaskManager::NonSignerStakesAndSignature {
                nonSignerPubkeys: non_signer_pubkeys,
                quorumApks: quorum_apks,
                apkG2: bls_agg_service_resp.signers_apk_g2,
                sigma: bls_agg_service_resp.signers_agg_sig_g1,
                nonSignerQuorumBitmapIndices: bls_agg_service_resp.non_signer_quorum_bitmap_indices,
                quorumApkIndices: bls_agg_service_resp.quorum_apk_indices,
                totalStakeIndices: bls_agg_service_resp.total_stake_indices,
                nonSignerStakeIndices: bls_agg_service_resp.non_signer_stake_indices,
            };

        self.logger.info(
            "Threshold reached. Sending aggregated response onchain.",
            &format!("task_index: {}", bls_agg_service_resp.task_index),
        );

        let task = {
            let tasks = self.tasks.read().unwrap();
            tasks.get(&bls_agg_service_resp.task_index).cloned()
        };

        let task_response = {
            let task_responses = self.task_responses.read().unwrap();
            task_responses
                .get(&bls_agg_service_resp.task_index)
                .and_then(|responses| responses.get(&bls_agg_service_resp.task_response_digest))
                .cloned()
        };

        if let (Some(task), Some(task_response)) = (task, task_response) {
            if let Err(err) = self
                .avs_writer
                .send_aggregated_response(task, task_response, non_signer_stakes_and_signature)
                .await
            {
                self.logger
                    .error("Aggregator failed to respond to task", &err.to_string());
            }
        }
    }
}
