use blueprint_sdk::extract::Context;
use blueprint_sdk::macros::context::{ServicesContext, TangleClientContext};
use blueprint_sdk::runner::config::BlueprintEnvironment;
use blueprint_sdk::tangle::extract::TangleArg;
use std::convert::Infallible;

#[cfg(test)]
mod tests;

#[derive(Clone, TangleClientContext, ServicesContext)]
pub struct MyContext {
    #[config]
    pub env: BlueprintEnvironment,
    #[call_id]
    pub call_id: Option<u64>,
}

/// Returns x^2 saturating to [`u64::MAX`] if overflow occurs.
pub async fn xsquare(
    Context(_context): Context<MyContext>,
    TangleArg(x): TangleArg<u64>,
) -> Result<u64, Infallible> {
    Ok(x.saturating_pow(2))
}
