use alloy_primitives::Address;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::types::OperatorInfo;

pub mod graphql;
pub mod in_memory;

#[derive(Debug, Clone)]
pub struct OperatorInfoService {
    pub operator_info_map: Arc<Mutex<HashMap<Address, OperatorInfo>>>,
}

impl OperatorInfoService {
    pub fn new() -> Self {
        OperatorInfoService {
            operator_info_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_operator_info(&self, operator: Address, info: OperatorInfo) {
        let mut map = self.operator_info_map.lock().await;
        map.insert(operator, info);
    }
}

#[async_trait]
pub trait OperatorInfoServiceTrait: Send + Sync + Clone + 'static {
    async fn get_operator_info(&self, operator: Address) -> Option<OperatorInfo>;
}

#[async_trait]
impl OperatorInfoServiceTrait for OperatorInfoService {
    async fn get_operator_info(&self, operator: Address) -> Option<OperatorInfo> {
        let map = self.operator_info_map.lock().await;
        map.get(&operator).cloned()
    }
}
