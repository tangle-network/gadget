use std::sync::Arc;
use crate::prelude::PalletSubmitter;
use crate::transaction_manager::TransactionManagerT;

pub type TangleTransactionManager = Arc<dyn PalletSubmitter>;

impl TransactionManagerT for TangleTransactionManager {
    fn transaction_manager(&self) -> &Self {
        self
    }
}