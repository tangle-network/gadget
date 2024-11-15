use crate::context::DfnsContext;
use cggmp21::key_share::AnyKeyShare;
use cggmp21::security_level::SecurityLevel128;
use cggmp21::supported_curves::Secp256k1;
use cggmp21::{DataToSign, ExecutionId};
use color_eyre::eyre::OptionExt;
use gadget_sdk::event_listener::tangle::jobs::{services_post_processor, services_pre_processor};
use gadget_sdk::event_listener::tangle::TangleEventListener;
use gadget_sdk::network::round_based_compat::NetworkDeliveryWrapper;
use gadget_sdk::network::StreamKey;
use gadget_sdk::random::rand::seq::SliceRandom;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use gadget_sdk::{compute_sha256_hash, job};
use k256::sha2::Sha256;
use rand_chacha::rand_core::SeedableRng;
use round_based::runtime::TokioRuntime;
use sp_core::ecdsa::Public;
use std::collections::BTreeMap;

#[job(
    id = 0,
    params(n, message_to_sign),
    event_listener(
        listener = TangleEventListener<DfnsContext, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ),
)]
/// Runs a [t; n] keygen using DFNS-CGGMP21. Returns the public key
pub async fn signing(
    n: u16,
    message_to_sign: Vec<u8>,
    context: DfnsContext,
) -> Result<Vec<u8>, gadget_sdk::Error> {
    let blueprint_id = context.blueprint_id()?;
    let call_id = context.current_call_id().await?;
    let meta_deterministic_hash = compute_sha256_hash!(
        n.to_be_bytes(),
        blueprint_id.to_be_bytes(),
        call_id.to_be_bytes(),
        "dfns"
    );
    let deterministic_hash = compute_sha256_hash!(meta_deterministic_hash, "dfns-signing");
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
    let keygen_output = context
        .store
        .get(&key)
        .ok_or_eyre("Keygen output not found in DB")?
        .refreshed_key
        .ok_or_eyre("Keygen output not found")?;
    // Choose `t` signers to perform signing
    let t = keygen_output.min_signers();
    let shares = &keygen_output.public_shares;
    let mut participants = (0..n).collect::<Vec<_>>();
    participants.shuffle(&mut rng);
    let participants = &participants[..usize::from(t)];
    println!("Signers: {participants:?}");
    let participants_shares = participants.iter().map(|i| &shares[usize::from(*i)]);

    // TODO: Parameterize the Curve type
    let signing =
        cggmp21::signing::<Secp256k1, SecurityLevel128>(eid, i as _, participants, &keygen_output)
            .enforce_reliable_broadcast(true);
    let message_to_sign = DataToSign::<Secp256k1>::digest::<Sha256>(&message_to_sign);
    let signature = signing
        .sign(&mut rng, party, message_to_sign)
        .await
        .map_err(|err| gadget_sdk::Error::Other(err.to_string()))?;

    let public_key = &keygen_output.shared_public_key;

    signature
        .verify(public_key, &message_to_sign)
        .map_err(|err| gadget_sdk::Error::Other(err.to_string()))?;

    let serialized_signature =
        bincode2::serialize(&signature).expect("Failed to serialize signature");

    Ok(serialized_signature)
}
