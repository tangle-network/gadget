use crate::client::TanglePalletSubmitter;

pub trait TransactionManager: Send + Sync + Clone + 'static {
    fn transaction_manager(&self) -> &Self {
        self
    }
}

impl<T: TanglePalletSubmitter + Clone> TransactionManager for T {}
