#![allow(clippy::too_many_arguments)]
use color_eyre::Result;
use gadget_config::{load, ContextConfig};
use gadget_clients::tangle::client::{BlueprintId, TangleClient};
use gadget_crypto::sp_core::SpSr25519;
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::{Keystore, KeystoreConfig};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_arithmetic::per_things::Percent;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::types::{
    Asset, AssetSecurityCommitment, AssetSecurityRequirement, MembershipModel,
};
use tangle_subxt::subxt::tx::{Signer, TxProgress};
use tangle_subxt::subxt::Config;
use tangle_subxt::subxt::blocks::ExtrinsicEvents;
use tangle_subxt::subxt::client::OnlineClientT;
use tangle_subxt::subxt::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::assets::events::created::AssetId;
use tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use gadget_config::supported_chains::SupportedChains;
use gadget_keystore::backends::Backend;
use gadget_utils_tangle::TxProgressExt;

pub async fn list_requests(_http_rpc_url: String, _ws_rpc_url: String) -> Result<()> {
    // TODO: Implement list requests functionality
    Ok(())
}

pub async fn accept_request(
    http_rpc_url: String,
    ws_rpc_url: String,
    service_id: Option<u64>,
    blueprint_id: u64,
    _min_exposure_percent: u8,
    _max_exposure_percent: u8,
    restaking_percent: u8,
    keystore_uri: String,
    keystore_password: Option<String>,
    chain: SupportedChains,
    request_id: u64,
) -> Result<()> {
    let config = ContextConfig::create_tangle_config(
        http_rpc_url.parse().unwrap(),
        ws_rpc_url.parse().unwrap(),
        keystore_uri,
        keystore_password,
        chain,
        blueprint_id,
        service_id,
    );
    let config = load(config).unwrap();
    let client = TangleClient::new(config.clone()).await.unwrap();
    let config = KeystoreConfig::new().fs_root(config.keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    let native_security_commitments =
        vec![get_security_commitment(Asset::Custom(0), restaking_percent)];

    let call = tangle_subxt::tangle_testnet_runtime::api::tx()
        .services()
        .approve(request_id, native_security_commitments);
    let res = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call, &signer)
        .await?;
    wait_for_in_block_success(res).await;
    Ok(())
}

pub async fn reject_request(
    http_rpc_url: String,
    ws_rpc_url: String,
    service_id: Option<u64>,
    blueprint_id: u64,
    keystore_uri: String,
    keystore_password: Option<String>,
    chain: SupportedChains,
    request_id: u64,
) -> Result<()> {
    let config = ContextConfig::create_tangle_config(
        http_rpc_url.parse().unwrap(),
        ws_rpc_url.parse().unwrap(),
        keystore_uri,
        keystore_password,
        chain,
        blueprint_id,
        service_id,
    );
    let config = load(config).unwrap();
    let client = TangleClient::new(config.clone()).await.unwrap();
    let config = KeystoreConfig::new().fs_root(config.keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    let call = tangle_subxt::tangle_testnet_runtime::api::tx()
        .services()
        .reject(request_id);
    let res = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call, &signer)
        .await?;
    wait_for_in_block_success(res).await;
    Ok(())
}

pub async fn request_service(
    http_rpc_url: String,
    ws_rpc_url: String,
    service_id: Option<u64>,
    blueprint_id: u64,
    min_exposure_percent: u8,
    max_exposure_percent: u8,
    target_operators: Vec<AccountId32>,
    value: u128,
    keystore_uri: String,
    keystore_password: Option<String>,
    chain: SupportedChains,
) -> Result<()> {
    let config = ContextConfig::create_tangle_config(
        http_rpc_url.parse().unwrap(),
        ws_rpc_url.parse().unwrap(),
        keystore_uri,
        keystore_password,
        chain,
        blueprint_id,
        service_id,
    );
    let config = load(config).unwrap();
    let client = TangleClient::new(config.clone()).await.unwrap();
    let config = KeystoreConfig::new().fs_root(config.keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    let min_operators = 0u32;
    let security_requirements = vec![AssetSecurityRequirement {
        asset: Asset::Custom(0),
        min_exposure_percent: Percent(min_exposure_percent),
        max_exposure_percent: Percent(max_exposure_percent),
    }];
    let call = tangle_subxt::tangle_testnet_runtime::api::tx()
        .services()
        .request(
            None,
            blueprint_id as BlueprintId,
            Vec::new(),
            target_operators,
            Default::default(),
            security_requirements,
            1000,
            Asset::Custom(0),
            value,
            MembershipModel::Fixed { min_operators },
        );
    let res = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call, &signer)
        .await?;
    wait_for_in_block_success(res).await;
    Ok(())
}

pub async fn submit_job(
    http_rpc_url: String,
    ws_rpc_url: String,
    service_id: Option<u64>,
    blueprint_id: u64,
    keystore_uri: String,
    keystore_password: Option<String>,
    chain: SupportedChains,
    job: u8,
) -> Result<()> {
    let config = ContextConfig::create_tangle_config(
        http_rpc_url.parse().unwrap(),
        ws_rpc_url.parse().unwrap(),
        keystore_uri,
        keystore_password,
        chain,
        blueprint_id,
        service_id,
    );
    let config = load(config).unwrap();
    let client = TangleClient::new(config.clone()).await.unwrap();
    let config = KeystoreConfig::new().fs_root(config.keystore_uri.clone());
    let keystore = Keystore::new(config).expect("Failed to create keystore");
    let public = keystore.first_local::<SpSr25519>().unwrap();
    let pair = keystore.get_secret::<SpSr25519>(&public).unwrap();
    let signer = TanglePairSigner::new(pair.0);

    let service_id = service_id.unwrap();
    let call = api::tx().services().call(service_id, job, vec![]); // TODO: Add inputs
    let events = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call, &signer)
        .await?
        .wait_for_finalized_success()
        .await?;

    let job_called_events = events.find::<JobCalled>().collect::<Vec<_>>();
    for job_called in job_called_events {
        let job_called = job_called?;
        if job_called.service_id == service_id
            && job_called.job == job
            && signer.account_id() == job_called.caller
        {
            return Ok(());
        }
    }
    panic!("Job was not called");
}

async fn wait_for_in_block_success<T: Config>(
    res: TxProgress<T, impl OnlineClientT<T>>,
) -> ExtrinsicEvents<T> {
    res.wait_for_in_block()
        .await
        .unwrap()
        .fetch_events()
        .await
        .unwrap()
}

fn get_security_commitment(a: Asset<AssetId>, p: u8) -> AssetSecurityCommitment<AssetId> {
    AssetSecurityCommitment {
        asset: a,
        exposure_percent: Percent(p),
    }
}
