use alloy_network::Ethereum;
use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use alloy_transport::Transport;
use async_trait::async_trait;
use std::collections::HashMap;
use std::iter::zip;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::task;

use crate::avs_registry::reader::AvsRegistryChainReader;
use crate::avs_registry::subscriber::AvsRegistryChainSubscriber;
use crate::crypto::bls::G1Point;
use crate::types::{operator_id_from_g1_pubkey, OperatorId, OperatorInfo, OperatorPubkeys, Socket};

use super::OperatorInfoServiceTrait;

const DEFAULT_LOG_FILTER_QUERY_BLOCK_RANGE: u64 = 10_000;

#[derive(Debug, Clone)]
pub struct OperatorsInfoServiceInMemory<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone + 'static,
{
    log_filter_query_block_range: u64,
    avs_registry_subscriber: AvsRegistryChainSubscriber<T, P>,
    avs_registry_reader: AvsRegistryChainReader<T, P>,
    query_sender: Sender<Query>,
    pubkey_dict: Arc<Mutex<HashMap<Address, OperatorPubkeys>>>,
    operator_addr_to_id: Arc<Mutex<HashMap<Address, OperatorId>>>,
    socket_dict: Arc<Mutex<HashMap<OperatorId, Socket>>>,
}

struct Query {
    operator_addr: Address,
    resp_sender: oneshot::Sender<Resp>,
}

struct Resp {
    operator_info: OperatorInfo,
    operator_exists: bool,
}

impl<T, P> OperatorsInfoServiceInMemory<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone + 'static,
{
    pub fn new(
        avs_registry_subscriber: AvsRegistryChainSubscriber<T, P>,
        avs_registry_reader: AvsRegistryChainReader<T, P>,
        log_filter_query_block_range: Option<u64>,
    ) -> Self {
        let (query_sender, query_receiver) = mpsc::channel(100);
        let log_filter_query_block_range =
            log_filter_query_block_range.unwrap_or(DEFAULT_LOG_FILTER_QUERY_BLOCK_RANGE);

        let service = OperatorsInfoServiceInMemory {
            avs_registry_subscriber,
            avs_registry_reader,
            log_filter_query_block_range,
            query_sender,
            pubkey_dict: Arc::new(Mutex::new(HashMap::new())),
            operator_addr_to_id: Arc::new(Mutex::new(HashMap::new())),
            socket_dict: Arc::new(Mutex::new(HashMap::new())),
        };

        service.start_service_in_task(query_receiver);

        service
    }

    fn start_service_in_task(self, mut query_receiver: Receiver<Query>) {
        let avs_registry_subscriber = self.avs_registry_subscriber.clone();
        let avs_registry_reader = self.avs_registry_reader.clone();
        let pubkey_dict = self.pubkey_dict.clone();
        let operator_addr_to_id = self.operator_addr_to_id.clone();
        let socket_dict = self.socket_dict.clone();

        task::spawn(async move {
            let mut new_pubkey_registration_stream = avs_registry_subscriber
                .subscribe_to_new_pubkey_registrations()
                .await
                .unwrap();
            let mut new_socket_registration_stream = avs_registry_subscriber
                .subscribe_to_operator_socket_updates()
                .await
                .unwrap();

            // Fill the pubkey_dict db with the operators and pubkeys found
            if let Err(e) = query_past_registered_operator_events_and_fill_db::<T, P>(
                &avs_registry_reader,
                &pubkey_dict,
                &operator_addr_to_id,
                &socket_dict,
                self.log_filter_query_block_range,
            )
            .await
            {
                log::error!("Error querying past registered operator events: {:?}", e);
                panic!("Error querying past registered operator events");
            }

            while let Some(event) = query_receiver.recv().await {
                match event {
                    Query {
                        operator_addr,
                        resp_sender,
                    } => {
                        let pubkeys = pubkey_dict.lock().unwrap().get(&operator_addr);
                        let operator_id = operator_addr_to_id
                            .lock()
                            .unwrap()
                            .get(&operator_addr)
                            .cloned();
                        let socket = operator_id
                            .as_ref()
                            .and_then(|id| socket_dict.lock().unwrap().get(id))
                            .cloned();
                        let operator_info = OperatorInfo {
                            socket: socket.unwrap_or_default(),
                            pubkeys: if pubkeys.is_some() {
                                pubkeys.unwrap().clone()
                            } else {
                                OperatorPubkeys::default()
                            },
                        };
                        let operator_exists = pubkeys.is_some();
                        let _ = resp_sender.send(Resp {
                            operator_info,
                            operator_exists,
                        });
                    }
                }
            }
        });
    }
}

pub async fn query_past_registered_operator_events_and_fill_db<T, P>(
    avs_registry_reader: &AvsRegistryChainReader<T, P>,
    pubkey_dict: &Arc<Mutex<HashMap<Address, OperatorPubkeys>>>,
    operator_addr_to_id: &Arc<Mutex<HashMap<Address, OperatorId>>>,
    socket_dict: &Arc<Mutex<HashMap<OperatorId, Socket>>>,
    log_filter_query_block_range: u64,
) -> Result<(), String>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    let already_registered_operator_addrs = avs_registry_reader
        .query_existing_registered_operator_pubkeys(0, 0, log_filter_query_block_range)
        .await
        .map_err(|e| e.to_string())?;
    let sockets_map = avs_registry_reader
        .query_existing_registered_operator_sockets(0, 0, log_filter_query_block_range)
        .await
        .map_err(|e| e.to_string())?;

    let (op_addrs_vec, op_pubkeys_vec) = already_registered_operator_addrs;
    for (operator_addr, operator_pubkeys) in zip(op_addrs_vec, op_pubkeys_vec) {
        let mut pubkey_dict = pubkey_dict.lock().unwrap();
        let mut operator_addr_to_id = operator_addr_to_id.lock().unwrap();
        let operator_id = operator_id_from_g1_pubkey(&G1Point {
            x: U256::from_limbs(operator_pubkeys.g1_pubkey.x.0 .0),
            y: U256::from_limbs(operator_pubkeys.g1_pubkey.y.0 .0),
        });
        pubkey_dict.insert(operator_addr, operator_pubkeys.clone());
        operator_addr_to_id.insert(operator_addr, operator_id.clone());
        log::debug!(
            "Added operator pubkeys to pubkey dict: {:?}",
            operator_pubkeys
        );
    }

    for (operator_id, socket) in sockets_map {
        let mut socket_dict = socket_dict.lock().unwrap();
        socket_dict.insert(operator_id.clone(), socket.clone());
        log::debug!("Added socket to socket dict: {:?}", socket);
    }

    Ok(())
}

#[async_trait]
impl<T, P> OperatorInfoServiceTrait for OperatorsInfoServiceInMemory<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    async fn get_operator_info(&self, operator_addr: Address) -> Option<OperatorInfo> {
        let (resp_sender, resp_receiver) = oneshot::channel();
        self.query_sender
            .send(Query {
                operator_addr,
                resp_sender,
            })
            .await
            .unwrap();
        resp_receiver.await.ok().map(|resp| resp.operator_info)
    }
}
