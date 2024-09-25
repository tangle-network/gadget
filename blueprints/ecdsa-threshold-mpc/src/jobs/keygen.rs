use cggmp21::{
    progress::PerfProfiler,
    supported_curves::{Secp256k1, Secp256r1, Stark},
};
use gadget_sdk::{
    self as sdk,
    network::{channels::UserID, IdentifierInfo, ProtocolMessage},
};
use generic_ec::Curve;
use rand::SeedableRng;
use sdk::tangle_subxt::subxt::ext::futures::future::try_join_all;
use sp_core::{ecdsa, keccak_256};
use std::sync::Arc;
use std::{collections::HashMap, convert::Infallible, fmt::format};
use tokio::spawn;

use color_eyre::Result;

use crate::mpc::{keygen::run_full_keygen_protocol, DefaultCryptoHasher, DefaultSecurityLevel};

use super::Context;

/// The execution of these jobs represent a service instance with specific operators.
///
/// Things that are appealing to provide the developer are:
/// - For networked protocols we need a unique identifier for protocol execution. Each job/task should have a unique identifier.
///   If multiple messaging protocols are created in a single task, they should have unique identifiers. This should be clearly
///   provided/exposed to the developer using our SDK. The variable names should likely indicate uniqueness.
#[sdk::job(
    id = 0,
    params(curve, t, num_keys, hd_wallet),
    result(_),
    verifier(evm = "HelloBlueprint")
)]
pub async fn keygen(
    ctx: Context,
    curve: u8,
    t: u16,
    num_keys: u16,
    hd_wallet: bool,
) -> Result<String, crate::mpc::Error> {
    // TODO: How to grab the specific operators? What index am I?
    let n = 3 * (t + 1);
    let i = 0;
    // TODO: Constructing mapping from user IDs to ecdsa keys?
    let mapping: HashMap<UserID, ecdsa::Public> = HashMap::new();
    // TODO: How to get my ecdsa key?
    let my_ecdsa_key = ecdsa::Public::from_raw([0u8; 33]);
    // TODO: How to grab api / task / job specific onchain metadata?
    // TODO: How to get restake information about this instance?
    // TODO: How to get the service ID?
    let service_id = 1u64;
    let (session_id, block_id, task_id, retry_id) =
        (Some(0u64), Some(0u64), Some(0u64), Some(0u64));
    let identifier_info: IdentifierInfo = IdentifierInfo {
        session_id,
        block_id,
        task_id,
        retry_id,
    };
    let rng = rand::rngs::StdRng::from_entropy();
    // TODO: How to get the raw job bytes?
    let job_id_bytes = vec![0u8; 32];
    let mix = keccak_256(b"cggmp21-keygen");
    let eid_bytes = [&job_id_bytes[..], &mix[..]].concat();
    let eid = cggmp21::ExecutionId::new(&eid_bytes);
    let mix = keccak_256(b"cggmp21-keygen-aux");
    let aux_eid_bytes = [&job_id_bytes[..], &mix[..]].concat();
    let aux_eid = cggmp21::ExecutionId::new(&aux_eid_bytes);
    let tracer = PerfProfiler::new();

    let id = service_id.to_string();
    let _span = tracing::info_span!("cggmp21-keygen", id = id).entered();

    // TODO: What is the protocol message channel? Can I get it from the network?
    let (_dummy_sender, protocol_message_channel) =
        futures::channel::mpsc::unbounded::<ProtocolMessage>();
    let mut handles = Vec::new();
    macro_rules! run_keygen_for_curve {
        ($curve:ty) => {
            for _ in 0..num_keys {
                let handle = spawn(async move {
                    run_full_keygen_protocol::<
                        $curve,
                        DefaultSecurityLevel,
                        DefaultCryptoHasher,
                        _,
                        _,
                    >(
                        protocol_message_channel,
                        identifier_info,
                        Arc::new(mapping),
                        my_ecdsa_key,
                        ctx.network,
                        tracer,
                        eid,
                        aux_eid,
                        i,
                        n,
                        t,
                        hd_wallet,
                        rng,
                        &job_id_bytes[..],
                    )
                    .await
                });
                handles.push(handle);
            }
        };
    }

    match curve {
        0 => run_keygen_for_curve!(Secp256k1),
        1 => run_keygen_for_curve!(Secp256r1),
        2 => run_keygen_for_curve!(Stark),
        _ => panic!("Invalid curve"),
    }

    let results = try_join_all(handles).await?;

    Ok("Hello World!".to_string())
}
