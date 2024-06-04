use crate::prelude::PalletSubmitter;
use crate::transaction_manager::TransactionManager;
use std::sync::Arc;

pub type TangleTransactionManager = Arc<dyn PalletSubmitter>;

impl TransactionManager for TangleTransactionManager {
    fn transaction_manager(&self) -> &Self {
        self
    }
}
