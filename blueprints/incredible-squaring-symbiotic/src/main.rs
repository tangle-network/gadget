use alloy_network::EthereumWallet;
use color_eyre::Result;
use gadget_sdk::runners::symbiotic::SymbioticConfig;
use gadget_sdk::runners::BlueprintRunner;
use gadget_sdk::{info, keystore::BackendExt};
use incredible_squaring_blueprint_symbiotic::{self as blueprint, IncredibleSquaringTaskManager};

use alloy_primitives::{address, Address};
use gadget_sdk::utils::evm::get_wallet_provider_http;
use lazy_static::lazy_static;
use std::env;

// Environment variables with default values
lazy_static! {
    pub static ref TASK_MANAGER_ADDRESS: Address = env::var("TASK_MANAGER_ADDRESS")
        .map(|addr| addr.parse().expect("Invalid TASK_MANAGER_ADDRESS"))
        .unwrap_or_else(|_| address!("0000000000000000000000000000000000000000"));
}

#[gadget_sdk::main(env)]
async fn main() {
    let operator_signer = env.keystore()?.ecdsa_key()?.alloy_key()?;
    let wallet = EthereumWallet::new(operator_signer);
    let provider = get_wallet_provider_http(&env.http_rpc_endpoint, wallet);

    let contract = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(
        *TASK_MANAGER_ADDRESS,
        provider,
    );

    let x_square = blueprint::XsquareEventHandler {
        context: blueprint::MyContext {},
        contract: contract.clone(),
        contract_instance: Default::default(),
    };

    info!("~~~ Executing the incredible squaring blueprint ~~~");
    let symb_config = SymbioticConfig::default();
    BlueprintRunner::new(symb_config, env)
        .job(x_square)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
