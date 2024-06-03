pub mod tangle;

pub trait TransactionManagerT: Send + Sync + 'static {
    fn transaction_manager(&self) -> &Self;
}
