use alloy_network::EthereumWallet;
use alloy_provider::ProviderBuilder;
use color_eyre::Result;
use gadget_sdk::events_watcher::evm::DefaultNodeConfig;
use gadget_sdk::runners::symbiotic::SymbioticConfig;
use gadget_sdk::runners::BlueprintRunner;
use gadget_sdk::{info, keystore::BackendExt};
use incredible_squaring_blueprint_symbiotic::{self as blueprint, IncredibleSquaringTaskManager};

use alloy_primitives::{address, Address};
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
    // Get the ECDSA key from the private key seed using alloy
    let operator_signer = env.keystore()?.ecdsa_key()?.alloy_key()?;
    let wallet = EthereumWallet::new(operator_signer);

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet.clone())
        .on_http(env.http_rpc_endpoint.parse()?);

    let contract = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(
        *TASK_MANAGER_ADDRESS,
        provider,
    );

    let x_square = blueprint::XsquareEventHandler::<DefaultNodeConfig> {
        context: blueprint::MyContext {},
        contract: contract.clone().into(),
    };

    info!("~~~ Executing the incredible squaring blueprint ~~~");
    let symb_config = SymbioticConfig {};
    BlueprintRunner::new(symb_config, env)
        .job(x_square)
        .run()
        .await?;

    info!("Exiting...");
    Ok(())
}
