use crate::env::StdGadgetConfiguration;

#[async_trait::async_trait]
pub trait GadgetRunner {
    type Error;

    fn env(&self) -> &StdGadgetConfiguration;
    async fn register(&self) -> Result<(), Self::Error>;
    async fn benchmark(&self) -> Result<(), Self::Error>;
    async fn run(&self) -> Result<(), Self::Error>;
}
