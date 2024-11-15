use crate::context::DfnsContext;
use cggmp21::security_level::SecurityLevel128;
use cggmp21::supported_curves::Secp256k1;
use cggmp21::{ExecutionId, PregeneratedPrimes};
use color_eyre::eyre::OptionExt;
use gadget_sdk::event_listener::tangle::jobs::{services_post_processor, services_pre_processor};
use gadget_sdk::event_listener::tangle::TangleEventListener;
use gadget_sdk::network::round_based_compat::NetworkDeliveryWrapper;
use gadget_sdk::network::StreamKey;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use gadget_sdk::{compute_sha256_hash, job};
use rand_chacha::rand_core::{RngCore, SeedableRng};
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
/// Runs a [t; n] keygen using DFNS-CGGMP21. Returns the public key
pub async fn key_refresh(n: u16, context: DfnsContext) -> Result<Vec<u8>, gadget_sdk::Error> {
    let blueprint_id = context.blueprint_id()?;
    let call_id = context.current_call_id().await?;
    let meta_deterministic_hash = compute_sha256_hash!(
        n.to_be_bytes(),
        blueprint_id.to_be_bytes(),
        call_id.to_be_bytes(),
        "dfns"
    );
    let deterministic_hash = compute_sha256_hash!(meta_deterministic_hash, "dfns-key-refresh");
    // TODO: make sure it's okay to have a different execution id for signing vs keygen even if same between for all keygen or all signing
    let eid = ExecutionId::new(&deterministic_hash);
    let (i, operators) = context.get_party_index_and_operators().await?;
    let parties: BTreeMap<u16, Public> = operators
        .into_iter()
        .enumerate()
        .map(|(j, (_, ecdsa))| (j as u16, ecdsa))
        .collect();

    gadget_sdk::info!(
        "Starting DFNS-CGGMP21 Signing for party {i}, n={n}, eid={}",
        hex::encode(eid.as_bytes())
    );

    let mut rng = rand_chacha::ChaChaRng::from_seed(deterministic_hash);
    let network = context.network_backend.multiplex(StreamKey {
        task_hash: deterministic_hash,
        round_id: 0,
    });
    let delivery = NetworkDeliveryWrapper::new(network, i as _, deterministic_hash, parties);
    let party = round_based::party::MpcParty::connected(delivery).set_runtime(TokioRuntime);

    let key = hex::encode(meta_deterministic_hash);
    let mut cggmp21_state = context
        .store
        .get(&key)
        .ok_or_eyre("Keygen output not found in DB")?;
    let keygen_output = cggmp21_state
        .inner
        .as_ref()
        .ok_or_eyre("Keygen output not found")?;

    // This generate_pregenerated_orimes function can take awhile to run
    let pregenerated_primes = generate_pregenerated_primes(rng.clone()).await?;

    // TODO: parameterize this
    let result = cggmp21::key_refresh::<Secp256k1, SecurityLevel128>(
        eid,
        keygen_output,
        pregenerated_primes,
    )
    .enforce_reliable_broadcast(true)
    .start(&mut rng, party)
    .await
    .map_err(|err| gadget_sdk::Error::Other(err.to_string()))?;

    cggmp21_state.refreshed_key = Some(result.clone());

    context.store.set(&key, cggmp21_state);

    let public_key =
        bincode2::serialize(&result.shared_public_key).expect("Failed to serialize public key");
    // TODO: Note: Earlier this year, bincode failed to serialize this DirtyKeyShare
    let serializable_share =
        bincode2::serialize(&result.into_inner()).expect("Failed to serialize share");

    Ok(public_key)
}

async fn generate_pregenerated_primes<R: RngCore + Send + 'static>(
    mut rng: R,
) -> Result<PregeneratedPrimes, gadget_sdk::Error> {
    let pregenerated_primes = tokio::task::spawn_blocking(move || {
        cggmp21::PregeneratedPrimes::<SecurityLevel128>::generate(&mut rng)
    })
    .await
    .map_err(|err| format!("Failed to generate pregenerated primes: {err:?}"))?;
    Ok(pregenerated_primes)
}
