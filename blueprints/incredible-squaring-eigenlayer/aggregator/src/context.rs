use crate::IncredibleSquaringTaskManager::{Task, TaskResponse};
use crate::NodeConfig;
use alloy_network::EthereumWallet;
use alloy_primitives::Address;
use eigensdk::types::avs::{TaskIndex, TaskResponseDigest};
use futures_util::lock::Mutex;
use gadget_sdk::ctx::KeystoreContext;
use gadget_sdk::{config::StdGadgetConfiguration, ctx::EigenlayerContext};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, EigenlayerContext, KeystoreContext)]
pub struct AggregatorContext {
    pub port_address: String,
    pub task_manager_addr: Address,
    pub tasks: Arc<Mutex<HashMap<TaskIndex, Task>>>,
    pub tasks_responses: Arc<Mutex<HashMap<TaskIndex, HashMap<TaskResponseDigest, TaskResponse>>>>,
    pub http_rpc_url: String,
    pub wallet: EthereumWallet,
    #[config]
    pub sdk_config: StdGadgetConfiguration,
}
