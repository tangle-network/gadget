pub mod tangle;

pub trait TransactionManager: Send + Sync + 'static {
    fn transaction_manager(&self) -> &Self;
}
