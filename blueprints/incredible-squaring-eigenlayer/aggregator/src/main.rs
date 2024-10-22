use color_eyre::Result;
use incredible_squaring_aggregator::{self, *};
use runner::execute_runner;

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> Result<()> {
    execute_runner().await
}
