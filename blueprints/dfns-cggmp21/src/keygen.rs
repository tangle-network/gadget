use crate::context::{DfnsContext, DfnsStore};
use cggmp21::supported_curves::Secp256k1;
use cggmp21::ExecutionId;
use gadget_sdk::event_listener::tangle::jobs::{services_post_processor, services_pre_processor};
use gadget_sdk::event_listener::tangle::TangleEventListener;
use gadget_sdk::network::round_based_compat::NetworkDeliveryWrapper;
use gadget_sdk::network::StreamKey;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use gadget_sdk::{compute_sha256_hash, job};
use rand_chacha::rand_core::SeedableRng;
use round_based::runtime::TokioRuntime;
use sp_core::ecdsa::Public;
use std::collections::BTreeMap;

#[job(
    id = 0,
    params(n),
    event_listener(
        listener = TangleEventListener<DfnsContext, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ),
)]
/// Runs a keygen using DFNS-CGGMP21. Returns the public key
pub async fn keygen(n: u16, context: DfnsContext) -> Result<Vec<u8>, gadget_sdk::Error> {
    let blueprint_id = context.blueprint_id()?;
    let call_id = context.current_call_id().await?;
    let meta_deterministic_hash = compute_sha256_hash!(
        n.to_be_bytes(),
        blueprint_id.to_be_bytes(),
        call_id.to_be_bytes(),
        "dfns"
    );
    let deterministic_hash = compute_sha256_hash!(meta_deterministic_hash, "dfns-keygen");
    let eid = ExecutionId::new(&deterministic_hash);
    let (i, operators) = context.get_party_index_and_operators().await?;
    let parties: BTreeMap<u16, Public> = operators
        .into_iter()
        .enumerate()
        .map(|(j, (_, ecdsa))| (j as u16, ecdsa))
        .collect();

    gadget_sdk::info!(
        "Starting DFNS-CGGMP21 Keygen for party {i}, n={n}, eid={}",
        hex::encode(eid.as_bytes())
    );

    let mut rng = rand_chacha::ChaChaRng::from_seed(deterministic_hash);
    let network = context.network_backend.multiplex(StreamKey {
        task_hash: deterministic_hash,
        round_id: 0,
    });
    let delivery = NetworkDeliveryWrapper::new(network, i as _, deterministic_hash, parties);
    let party = round_based::party::MpcParty::connected(delivery).set_runtime(TokioRuntime);

    // TODO: Parameterize the Curve type
    let result = cggmp21::keygen::<Secp256k1>(eid, i as u16, n)
        .enforce_reliable_broadcast(true)
        .start(&mut rng, party)
        .await
        .map_err(|err| gadget_sdk::Error::Other(err.to_string()))?;

    let key = hex::encode(meta_deterministic_hash);
    context.store.set(
        &key,
        DfnsStore {
            inner: Some(result.clone()),
            refreshed_key: None,
        },
    );

    let public_key =
        bincode2::serialize(&result.shared_public_key).expect("Failed to serialize public key");
    // TODO: Note: Earlier this year, bincode failed to serialize this DirtyKeyShare
    let serializable_share =
        bincode2::serialize(&result.into_inner()).expect("Failed to serialize share");

    Ok(public_key)
}
