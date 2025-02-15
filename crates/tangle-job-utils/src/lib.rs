extern crate alloc;

use blueprint_job_router::{BoxError, JobCall};
use bytes::Bytes;
use gadget_blueprint_serde::Field;
use tangle_subxt::parity_scale_codec::Encode;
use tangle_subxt::subxt::utils::AccountId32;

use producer::TangleClient;

pub mod extract;
pub mod filters;
pub mod layers;
pub mod producer;

#[bon::builder]
pub fn create_call(
    job_id: u32,
    call_id: u64,
    block_number: u64,
    service_id: u64,
    args: Option<Field<AccountId32>>,
) -> JobCall {
    let mut call = match args {
        Some(args) => JobCall::new(job_id, Bytes::from(args.encode())),
        None => JobCall::empty(job_id),
    };
    call.metadata_mut()
        .insert(extract::CallId::METADATA_KEY, call_id);

    call.metadata_mut()
        .insert(extract::BlockNumber::METADATA_KEY, block_number);

    call.metadata_mut()
        .insert(extract::ServiceId::METADATA_KEY, service_id);
    call
}

async fn deploy_blueprint(clinet: &TangleClient) -> Result<u64, BoxError> {
    use api::runtime_types::tangle_primitives::services::field::*;
    use api::runtime_types::tangle_primitives::services::*;
    use api::services::events::BlueprintCreated;
    use gadget_blueprint_serde::new_bounded_string;
    use gadget_blueprint_serde::BoundedVec;
    use tangle_subxt::subxt_signer::sr25519::dev;
    use tangle_subxt::tangle_testnet_runtime::api;

    let blueprint = ServiceBlueprint {
        metadata: ServiceMetadata {
            name: new_bounded_string("Incredible Squaring"),
            description: Some(new_bounded_string("A service that squares numbers")),
            author: None,
            category: None,
            code_repository: None,
            logo: None,
            website: None,
            license: None,
        },
        jobs: BoundedVec(vec![
            JobDefinition {
                metadata: JobMetadata {
                    name: new_bounded_string("square"),
                    description: Some(new_bounded_string("Squares a number")),
                },
                params: BoundedVec(vec![FieldType::Uint64]),
                result: BoundedVec(vec![FieldType::Uint64]),
            },
            JobDefinition {
                metadata: JobMetadata {
                    name: new_bounded_string("multiply"),
                    description: Some(new_bounded_string("Multiplies two numbers")),
                },
                params: BoundedVec(vec![FieldType::Array(2, Box::new(FieldType::Uint64))]),
                result: BoundedVec(vec![FieldType::Uint64]),
            },
        ]),
        registration_params: BoundedVec(Default::default()),
        request_params: BoundedVec(Default::default()),
        manager: BlueprintServiceManager::Evm(Default::default()),
        master_manager_revision: MasterBlueprintServiceManagerRevision::Latest,
        gadget: Gadget::Wasm(WasmGadget {
            runtime: WasmRuntime::Wasmtime,
            sources: BoundedVec(Default::default()),
        }),
    };
    let tx = api::tx().services().create_blueprint(blueprint);
    let signer = dev::alice();
    let mut x = clinet
        .tx()
        .sign_and_submit_then_watch_default(&tx, &signer)
        .await?;
    let block = loop {
        if let Some(Ok(status)) = x.next().await {
            match status {
                tangle_subxt::subxt::tx::TxStatus::InBestBlock(b) => {
                    break b;
                }
                _ => continue,
            }
        }
    };
    let ev = block
        .fetch_events()
        .await?
        .find_first::<BlueprintCreated>()?;
    ev.map(|e| e.blueprint_id)
        .ok_or_else(|| "No BlueprintCreated event".into())
}

async fn register_on_blueprint(clinet: &TangleClient, blueprint_id: u64) -> Result<(), BoxError> {
    use api::runtime_types::tangle_primitives::services::*;
    use api::services::events::Registered;
    use tangle_subxt::subxt_signer::sr25519::dev;
    use tangle_subxt::tangle_testnet_runtime::api;

    let tx = api::tx().services().register(
        blueprint_id,
        OperatorPreferences {
            key: [0; 65],
            price_targets: PriceTargets {
                cpu: 0,
                mem: 0,
                storage_hdd: 0,
                storage_ssd: 0,
                storage_nvme: 0,
            },
        },
        Default::default(),
        0,
    );
    let signer = dev::alice();
    let mut x = clinet
        .tx()
        .sign_and_submit_then_watch_default(&tx, &signer)
        .await?;
    let block = loop {
        if let Some(Ok(status)) = x.next().await {
            match status {
                tangle_subxt::subxt::tx::TxStatus::InBestBlock(b) => {
                    break b;
                }
                _ => continue,
            }
        }
    };
    block
        .fetch_events()
        .await?
        .find_first::<Registered>()?
        .map(|_| ())
        .ok_or_else(|| "No Registered event".into())
}

async fn request_service(clinet: &TangleClient, blueprint_id: u64) -> Result<u64, BoxError> {
    use api::runtime_types::tangle_primitives::services::*;
    use api::services::events::ServiceRequested;
    use tangle_subxt::subxt_signer::sr25519::dev;
    use tangle_subxt::tangle_testnet_runtime::api;

    let tx = api::tx().services().request(
        None,
        blueprint_id,
        Default::default(),
        vec![dev::alice().public_key().to_account_id()],
        Default::default(),
        Default::default(),
        100000,
        Asset::Custom(0),
        0,
    );

    let signer = dev::alice();
    let mut x = clinet
        .tx()
        .sign_and_submit_then_watch_default(&tx, &signer)
        .await?;
    let block = loop {
        if let Some(Ok(status)) = x.next().await {
            match status {
                tangle_subxt::subxt::tx::TxStatus::InBestBlock(b) => {
                    break b;
                }
                _ => continue,
            }
        }
    };

    let ev = block
        .fetch_events()
        .await?
        .find_first::<ServiceRequested>()?;

    ev.map(|e| e.request_id)
        .ok_or_else(|| "No ServiceRequested event".into())
}

async fn approve_request(clinet: &TangleClient, request_id: u64) -> Result<u64, BoxError> {
    use api::runtime_types::sp_arithmetic::per_things::Percent;
    use api::services::events::ServiceRequestApproved;
    use tangle_subxt::subxt_signer::sr25519::dev;
    use tangle_subxt::tangle_testnet_runtime::api;

    let tx = api::tx().services().approve(request_id, Percent(1));
    let signer = dev::alice();
    let mut x = clinet
        .tx()
        .sign_and_submit_then_watch_default(&tx, &signer)
        .await?;
    let block = loop {
        if let Some(Ok(status)) = x.next().await {
            match status {
                tangle_subxt::subxt::tx::TxStatus::InBestBlock(b) => {
                    break b;
                }
                _ => continue,
            }
        }
    };
    block
        .fetch_events()
        .await?
        .find_first::<ServiceRequestApproved>()?
        .map(|r| r.request_id)
        .ok_or_else(|| "No ServiceRequestApproved event".into())
}
