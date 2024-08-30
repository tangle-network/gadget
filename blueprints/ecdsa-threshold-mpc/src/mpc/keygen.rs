use cggmp21::generic_ec::Curve;
use cggmp21::key_refresh::msg::aux_only;
use cggmp21::key_refresh::AuxInfoGenerationBuilder;
use cggmp21::key_share::DirtyKeyShare;
use cggmp21::key_share::Validate;
use cggmp21::keygen::msg::threshold::Msg;
use cggmp21::keygen::KeygenBuilder;
use cggmp21::progress::PerfProfiler;
use cggmp21::security_level::SecurityLevel;
use cggmp21::supported_curves::{Secp256k1, Secp256r1, Stark};
use cggmp21::KeyShare;
use cggmp21::PregeneratedPrimes;
use digest::typenum::U32;
use digest::Digest;
use futures::channel::mpsc::{TryRecvError, UnboundedSender};
use futures::StreamExt;
use gadget_common::channels::UserID;
use gadget_common::config::DebugLogger;
use gadget_common::utils::{deserialize, serialize};
use gadget_common::JobError;
use gadget_common::WorkManagerInterface;
use gadget_sdk::network::Network;
use itertools::Itertools;
use rand::rngs::{OsRng, StdRng};
use rand::{CryptoRng, RngCore, SeedableRng};
use round_based::{Delivery, Incoming, MpcParty, Outgoing};
use serde::Serialize;
use sp_core::keccak_256;
use sp_core::{ecdsa, Pair};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::protocols::{DefaultCryptoHasher, DefaultSecurityLevel};

#[allow(clippy::too_many_arguments)]
pub async fn run_and_serialize_keygen<
    'r,
    E: Curve,
    S: SecurityLevel,
    H: Digest<OutputSize = U32> + Clone + Send + 'static,
    D,
    R,
>(
    logger: &DebugLogger,
    tracer: &mut PerfProfiler,
    eid: cggmp21::ExecutionId<'r>,
    i: u16,
    n: u16,
    t: u16,
    hd_wallet: bool,
    party: MpcParty<Msg<E, S, H>, D>,
    mut rng: R,
) -> Result<Vec<u8>, JobError>
where
    D: Delivery<Msg<E, S, H>>,
    R: RngCore + CryptoRng,
{
    let builder = KeygenBuilder::<E, S, H>::new(eid, i, n);
    let incomplete_key_share = builder
        .set_progress_tracer(tracer)
        .set_threshold(t)
        .hd_wallet(hd_wallet)
        .start(&mut rng, party)
        .await
        .map_err(|err| JobError {
            reason: format!("Keygen protocol error (run_and_serialize_keygen): {err:?}"),
        })?;
    logger.debug("Finished AsyncProtocol - Keygen");
    serialize(&incomplete_key_share).map_err(|err| JobError {
        reason: format!("Keygen protocol error (run_and_serialize_keygen - serde): {err:?}"),
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn run_and_serialize_keyrefresh<
    'r,
    E: Curve,
    S: SecurityLevel,
    H: Digest<OutputSize = U32> + Clone + Send + 'static,
    D,
>(
    logger: &DebugLogger,
    incomplete_key_share: Vec<u8>,
    pregenerated_primes: PregeneratedPrimes<S>,
    tracer: &mut PerfProfiler,
    aux_eid: cggmp21::ExecutionId<'r>,
    i: u16,
    n: u16,
    party: MpcParty<aux_only::Msg<H, S>, D>,
    mut rng: StdRng,
) -> Result<(Vec<u8>, Vec<u8>), JobError>
where
    D: Delivery<aux_only::Msg<H, S>>,
{
    let incomplete_key_share: cggmp21::key_share::Valid<
        cggmp21::key_share::DirtyIncompleteKeyShare<E>,
    > = deserialize(&incomplete_key_share).map_err(|err| JobError {
        reason: format!("Keygen protocol error (run_and_serialize_keyrefresh): {err:?}"),
    })?;

    let aux_info_builder =
        AuxInfoGenerationBuilder::<S, H>::new_aux_gen(aux_eid, i, n, pregenerated_primes);

    let aux_info = aux_info_builder
        .set_progress_tracer(tracer)
        .start(&mut rng, party)
        .await
        .map_err(|err| JobError {
            reason: format!("Aux info protocol error: {err:?}"),
        })?;
    let perf_report = tracer.get_report().map_err(|err| JobError {
        reason: format!("Aux info protocol error: {err:?}"),
    })?;
    logger.trace(format!("Aux info protocol report: {perf_report}"));
    logger.debug("Finished AsyncProtocol - Aux Info");

    let key_share: KeyShare<E, S> = DirtyKeyShare {
        core: incomplete_key_share.into_inner(),
        aux: aux_info.into_inner(),
    }
    .validate()
    .map_err(|err| JobError {
        reason: format!("Keygen protocol validation error: {err:?}"),
    })?;
    // Serialize the key share and the public key
    serialize(&key_share)
        .map(|ks| (ks, key_share.shared_public_key.to_bytes(true).to_vec()))
        .map_err(|err| JobError {
            reason: format!("Keygen protocol error (run_and_serialize_keyrefresh): {err:?}"),
        })
}

async fn pregenerate_primes<S: SecurityLevel, KBE: KeystoreBackend>(
    tracer: &PerfProfiler,
    logger: &DebugLogger,
    job_id_bytes: &[u8],
) -> Result<(PerfProfiler, PregeneratedPrimes<S>), JobError> {
    let perf_report = tracer.get_report().map_err(|err| JobError {
        reason: format!("Keygen protocol error (pregenerate_primes): {err:?}"),
    })?;
    logger.trace(format!("Incomplete Keygen protocol report: {perf_report}"));
    logger.debug("Finished AsyncProtocol - Incomplete Keygen");
    let tracer = PerfProfiler::new();

    let pregenerated_primes_key =
        keccak_256(&[&b"dfns-cggmp21-keygen-primes"[..], job_id_bytes].concat());
    let now = tokio::time::Instant::now();
    let pregenerated_primes = tokio::task::spawn_blocking(|| {
        let mut rng = OsRng;
        cggmp21::PregeneratedPrimes::<S>::generate(&mut rng)
    })
    .await
    .map_err(|err| JobError {
        reason: format!("Failed to generate pregenerated primes: {err:?}"),
    })?;

    let elapsed = now.elapsed();
    logger.debug(format!("Pregenerated primes took {elapsed:?}"));

    Ok((tracer, pregenerated_primes))
}

#[allow(clippy::too_many_arguments)]
pub async fn run_full_keygen_protocol<
    'a,
    E: Curve,
    S: SecurityLevel,
    H: Digest<OutputSize = U32> + Clone + Send + 'static,
    KBE: KeystoreBackend,
    N: Network,
>(
    protocol_message_channel: UnboundedReceiver<TangleProtocolMessage>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
    mapping: Arc<HashMap<UserID, ecdsa::Public>>,
    my_role_id: ecdsa::Public,
    network: N,
    mut tracer: PerfProfiler,
    eid: cggmp21::ExecutionId<'a>,
    aux_eid: cggmp21::ExecutionId<'a>,
    i: u16,
    n: u16,
    t: u16,
    hd_wallet: bool,
    rng: StdRng,
    logger: &DebugLogger,
    job_id_bytes: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), JobError> {
    let (tx0, rx0, tx1, rx1) =
        gadget_common::channels::create_job_manager_to_async_protocol_channel_split_io(
            protocol_message_channel,
            associated_block_id,
            associated_retry_id,
            associated_session_id,
            associated_task_id,
            mapping,
            my_role_id,
            network,
            logger.clone(),
            i,
        );
    let delivery = (rx0, tx0);
    let party = MpcParty::<Msg<E, S, H>, _, _>::connected(delivery);
    let incomplete_key_share = run_and_serialize_keygen::<E, S, H, _, _>(
        logger,
        &mut tracer,
        eid,
        i,
        n,
        t,
        hd_wallet,
        party,
        rng.clone(),
    )
    .await?;
    let (mut tracer, pregenerated_primes) =
        pregenerate_primes::<S, KBE>(&tracer, logger, job_id_bytes).await?;

    logger.info(format!("Will now run Keygen protocol: {role_type:?}"));

    let delivery = (rx1, tx1);
    let party = MpcParty::<aux_only::Msg<H, S>, _, _>::connected(delivery);
    let (key_share, serialized_public_key) = run_and_serialize_keyrefresh::<E, S, H, _>(
        logger,
        incomplete_key_share,
        pregenerated_primes,
        &mut tracer,
        aux_eid,
        i,
        n,
        party,
        rng,
    )
    .await?;

    Ok((key_share, serialized_public_key))
}
