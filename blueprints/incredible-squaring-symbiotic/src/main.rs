use blueprint_sdk::alloy::network::EthereumWallet;
use blueprint_sdk::logging::info;
use blueprint_sdk::main;
use blueprint_sdk::runners::core::runner::BlueprintRunner;
use blueprint_sdk::utils::evm::get_wallet_provider_http;
use incredible_squaring_blueprint_symbiotic::{self as blueprint, IncredibleSquaringTaskManager};
use std::env;

// Environment variables with default values
lazy_static! {
    pub static ref TASK_MANAGER_ADDRESS: Address = env::var("TASK_MANAGER_ADDRESS")
        .map(|addr| addr.parse().expect("Invalid TASK_MANAGER_ADDRESS"))
        .unwrap_or_else(|_| address!("0000000000000000000000000000000000000000"));
}

#[main(env)]
async fn main() {
    let operator_signer = env.keystore()?.ecdsa_key()?.alloy_key()?;
    let wallet = EthereumWallet::new(operator_signer);
    let provider = get_wallet_provider_http(&env.http_rpc_endpoint, wallet);

    let contract = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(
        *TASK_MANAGER_ADDRESS,
        provider,
    );

    let x_square = blueprint::XsquareEventHandler::new(contract, blueprint::MyContext {});

    info!("~~~ Executing the incredible squaring blueprint ~~~");
    let symb_config = SymbioticConfig::default();
    BlueprintRunner::new(symb_config, env)
        .job(x_square)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
