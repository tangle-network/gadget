use cggmp21::supported_curves::{Secp256k1, Secp256r1, Stark};
use gadget_sdk as sdk;
use generic_ec::Curve;
use sdk::tangle_subxt::subxt::ext::futures::future::try_join_all;
use std::convert::Infallible;
use tokio::spawn;

use color_eyre::Result;

use crate::mpc::keygen::run_full_keygen_protocol;

use super::Context;

/// The execution of these jobs represent a service instance with specific operators.
///
/// Things that are appealing to provide the developer are:
/// - For networked protocols we need a unique identifier for protocol execution. Each job/task should have a unique identifier.
///   If multiple messaging protocols are created in a single task, they should have unique identifiers. This should be clearly
///   provided/exposed to the developer using our SDK. The variable names should likely indicate uniqueness.
#[sdk::job(
    id = 0,
    params(parties, t, num_keys),
    result(_),
    verifier(evm = "HelloBlueprint")
)]
pub fn keygen(ctx: Context, curve: Curve, t: u16, num_keys: u16) -> Result<String, Infallible> {
    // TODO: How to grab the specific operators?
    let num_operators = t + 1;
    assert!(t > 0);
    assert!(num_operators > t);

    let mut handles = Vec::new();

    for _ in 0..num_keys {
        let handle = spawn(async move {
            run_full_keygen_protocol(
                protocol_message_channel,
                associated_block_id,
                associated_retry_id,
                associated_session_id,
                associated_task_id,
                mapping,
                my_role_id,
                network,
                tracer,
                eid,
                aux_eid,
                i,
                n,
                t,
                hd_wallet,
                rng,
                logger,
                &job_id_bytes,
            )
            .await
        });

        handles.push(handle);
    }

    let results = try_join_all(handles).await?;

    Ok("Hello World!".to_string())
}
