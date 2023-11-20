use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    pin::Pin,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use ark_circom::CircomReduction;
use ark_crypto_primitives::snark::SNARK;
use ark_ec::pairing::Pairing;
use ark_ec::CurveGroup;
use ark_groth16::{Groth16, ProvingKey};
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_relations::r1cs::SynthesisError;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{cfg_iter, start_timer};
use ark_std::{end_timer, Zero};
use clap::Parser;
use futures::{future, TryFutureExt};
use gadget_core::{gadget::substrate::Client, job_manager::SendFuture};
use groth16::proving_key::PackedProvingKeyShare;
use mpc_net::prod::CertToDer;
use mpc_net::MultiplexedStreamID;
use rustls::RootCertStore;
use sc_client_api::FinalizeSummary;
use sc_utils::mpsc::{TracingUnboundedReceiver, TracingUnboundedSender};
use secret_sharing::pss::PackedSharingParams;
use sp_runtime::app_crypto::sp_core::Encode;
use sp_runtime::codec::Decode;
use sp_runtime::traits::Block;
use tokio::sync::{mpsc::UnboundedSender, Mutex, Semaphore};
use tracing_subscriber::{fmt::SubscriberBuilder, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;
use webb_gadget::{BlockImportNotification, FinalityNotification};
use zk_gadget::{
    client_ext::{
        job_types::{CircuitProperties, JobProperties},
        ClientWithApi,
    },
    module::proto_gen::ZkAsyncProtocolParameters,
    network::ZkNetworkService,
};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

const BLOCK_DURATION: Duration = Duration::from_millis(6000);

type F = ark_bn254::Fr;
type E = ark_bn254::Bn254;

/// ZK Node
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the node, as it will appear in the logs
    #[arg(long)]
    name: String,
    /// The Id of the node, also acts as the party index
    #[arg(long)]
    i: usize,
    /// King's IP address
    #[arg(long)]
    king_ip: SocketAddr,
    /// Cuurent node's private identity DER
    #[arg(long)]
    private_identity_der: PathBuf,
    /// All the nodes' public identity DER (including the current node)
    /// it must be in the same order as the nodes' Ids
    #[arg(long)]
    certs: Vec<PathBuf>,
    /// Watch directory for new jobs
    #[arg(long)]
    watch_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_log();
    let args = Args::parse();
    assert!(
        args.i < args.certs.len(),
        "i must be less than n (certs.len())"
    );

    tracing::info!(node = args.name, "Starting zknode");

    let state = State {
        circuits: Arc::new(Mutex::new(HashMap::new())),
        jobs: Arc::new(Mutex::new(HashMap::new())),
        processing_semophore: Arc::new(Semaphore::new(1)),
        watch_dir: args.watch_dir,
    };
    let client = BlockchainClient::new(state.clone());
    let (done_tx, mut done_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    let mut cert_store = RootCertStore::empty();
    // add all certs except our own
    let iter = args.certs.iter().enumerate().filter(|(i, _)| *i != args.i);
    for (_, cert_path) in iter {
        let cert = rustls::Certificate(std::fs::read(cert_path)?);
        cert_store.add(&cert)?;
    }

    let gadget_config = if args.i == 0 {
        let (king_public_identity_der, king_private_identity_der) = {
            let king_public_identity_der_path = args
                .certs
                .get(args.i)
                .expect("King's public identity DER path is required");
            let king_private_identity_der_path = args.private_identity_der;
            (
                std::fs::read(king_public_identity_der_path)?,
                std::fs::read(king_private_identity_der_path)?,
            )
        };
        zk_gadget::ZkGadgetConfig {
            king_bind_addr: Some(args.king_ip),
            client_only_king_addr: None,
            id: args.i as _,
            n_parties: args.certs.len(),
            public_identity_der: king_public_identity_der.clone(),
            private_identity_der: king_private_identity_der.clone(),
            client_only_king_public_identity_der: None,
        }
    } else {
        let king_public_identity_der_path = args
            .certs
            .get(0)
            .expect("King's public identity DER path is required");
        let king_public_identity_der = std::fs::read(king_public_identity_der_path)?;
        // read our identity
        let (public_identity_der, private_identity_der) = {
            let public_identity_der_path = args
                .certs
                .get(args.i)
                .expect("Public identity DER path is required");
            let private_identity_der_path = args.private_identity_der;
            (
                std::fs::read(public_identity_der_path)?,
                std::fs::read(private_identity_der_path)?,
            )
        };
        zk_gadget::ZkGadgetConfig {
            king_bind_addr: None,
            client_only_king_addr: Some(args.king_ip),
            id: args.i as _,
            n_parties: args.certs.len(),
            public_identity_der,
            private_identity_der,
            client_only_king_public_identity_der: Some(king_public_identity_der.clone()),
        }
    };

    let additional_parameters = AdditionalParams {
        stop_tx: done_tx.clone(),
    };

    let zk_gadget_future = zk_gadget::run(
        gadget_config,
        client,
        additional_parameters,
        async_protocol_generator,
    );

    let done_rx_future = async move {
        done_rx.recv().await.ok_or("Did not receive done signal")?;
        tracing::info!("Received done signal");
        Ok::<_, String>(())
    };

    tokio::select! {
        res0 = zk_gadget_future => {
            res0?;
        },
        res1 = done_rx_future => {
            res1.map_err(|e| anyhow::anyhow!("{e}"))?
        }
    }

    Ok(())
}

#[derive(Clone)]
struct State {
    pub circuits: Arc<Mutex<HashMap<u64, CircuitProperties>>>,
    pub jobs: Arc<Mutex<HashMap<[u8; 32], JobProperties>>>,
    pub processing_semophore: Arc<Semaphore>,
    pub watch_dir: PathBuf,
}

#[derive(Clone)]
struct AdditionalParams {
    #[allow(unused)]
    stop_tx: UnboundedSender<()>,
}

fn async_protocol_generator(
    params: ZkAsyncProtocolParameters<
        AdditionalParams,
        ZkNetworkService,
        BlockchainClient,
        TestBlock,
    >,
) -> Pin<Box<dyn SendFuture<'static, Result<(), webb_gadget::Error>>>> {
    Box::pin(async move {
        let _permit = match params.client.state.processing_semophore.try_acquire() {
            Ok(permit) => permit,
            Err(tokio::sync::TryAcquireError::NoPermits) => {
                // No more processing power, we return.
                return Ok(());
            }
            Err(tokio::sync::TryAcquireError::Closed) => unreachable!("Never close"),
        };
        let jobs = params.client.state.jobs.lock().await;
        let circuits = params.client.state.circuits.lock().await;
        let Some(job) = jobs.get(&params.associated_task_id).cloned() else {
            return Err(webb_gadget::Error::ClientError {
                err: format!(
                    "Job with id {} not found",
                    hex::encode(params.associated_task_id)
                ),
            });
        };
        let Some(circuit) = circuits.get(&job.circuit_id).cloned() else {
            return Err(webb_gadget::Error::ClientError {
                err: format!("Circuit with id {} not found", job.circuit_id),
            });
        };

        drop(jobs);
        drop(circuits);

        let proving_key = tokio::fs::read(&circuit.pk_uri)
            .map_err(|e| webb_gadget::Error::ClientError {
                err: format!("Failed to read proving key: {e}"),
            })
            .await?;
        let pk = ProvingKey::deserialize_compressed(proving_key.as_slice()).map_err(|e| {
            webb_gadget::Error::ClientError {
                err: format!("Failed to deserialize proving key: {e}"),
            }
        })?;

        let pp = PackedSharingParams::new(job.pss_l);
        let pp_g1 = PackedSharingParams::new(pp.l);
        let pp_g2 = PackedSharingParams::new(pp.l);
        let crs_shares_timer = start_timer!(|| "CRS Shares");
        let crs_shares =
            PackedProvingKeyShare::<E>::pack_from_arkworks_proving_key(&pk, pp_g1, pp_g2);
        end_timer!(crs_shares_timer);
        let our_qap_share = job
            .qap_shares
            .get(params.party_id as usize)
            .ok_or_else(|| webb_gadget::Error::ClientError {
                err: format!("QAP Shares for party {} not found", params.party_id),
            })?;
        let our_a_share = job.a_shares.get(params.party_id as usize).ok_or_else(|| {
            webb_gadget::Error::ClientError {
                err: format!("a Shares for party {} not found", params.party_id),
            }
        })?;
        let our_ax_share = job.ax_shares.get(params.party_id as usize).ok_or_else(|| {
            webb_gadget::Error::ClientError {
                err: format!("ax Shares for party {} not found", params.party_id),
            }
        })?;
        let m = circuit.num_inputs + circuit.num_constraints;
        let domain = Radix2EvaluationDomain::new(m)
            .ok_or(SynthesisError::PolynomialDegreeTooLarge)
            .unwrap();
        let qap_share = groth16::qap::PackedQAPShare {
            num_inputs: circuit.num_inputs,
            num_constraints: circuit.num_constraints,
            a: cfg_iter!(our_qap_share.a)
                .map(|f| F::deserialize_compressed(f.as_slice()))
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            b: cfg_iter!(our_qap_share.b)
                .map(|f| F::deserialize_compressed(f.as_slice()))
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            c: cfg_iter!(our_qap_share.c)
                .map(|f| F::deserialize_compressed(f.as_slice()))
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            domain,
        };

        let crs_share = crs_shares.get(params.party_id as usize).ok_or_else(|| {
            webb_gadget::Error::ClientError {
                err: format!("CRS Shares for party {} not found", params.party_id),
            }
        })?;
        let a_share = cfg_iter!(our_a_share)
            .map(|f| F::deserialize_compressed(f.as_slice()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let ax_share = cfg_iter!(our_ax_share)
            .map(|f| F::deserialize_compressed(f.as_slice()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let h_share = groth16::ext_wit::h(qap_share, &pp, &params).await.unwrap();
        tracing::debug!("Finished Computing h_share");
        let msm_section = start_timer!(|| "MSM operations");
        // Compute msm while dropping the base vectors as they are not used again
        let compute_a = start_timer!(|| "Compute A");
        let pi_a_share = groth16::prove::A::<E> {
            L: Default::default(),
            N: Default::default(),
            r: <E as Pairing>::ScalarField::zero(),
            pp: &pp,
            S: &crs_share.s,
            a: &a_share,
        }
        .compute(&params, MultiplexedStreamID::Zero)
        .await
        .unwrap();
        end_timer!(compute_a);

        let compute_b = start_timer!(|| "Compute B");
        let pi_b_share = groth16::prove::B::<E> {
            Z: Default::default(),
            K: Default::default(),
            s: <E as Pairing>::ScalarField::zero(),
            pp: &pp,
            V: &crs_share.v,
            a: &a_share,
        }
        .compute(&params, MultiplexedStreamID::Zero)
        .await
        .unwrap();
        end_timer!(compute_b);

        let compute_c = start_timer!(|| "Compute C");
        let pi_c_share = groth16::prove::C::<E> {
            W: &crs_share.w,
            U: &crs_share.u,
            A: pi_a_share,
            M: Default::default(),
            r: <E as Pairing>::ScalarField::zero(),
            s: <E as Pairing>::ScalarField::zero(),
            pp: &pp,
            H: &crs_share.h,
            a: &a_share,
            ax: &ax_share,
            h: &h_share,
        }
        .compute(&params)
        .await
        .unwrap();
        end_timer!(compute_c);

        end_timer!(msm_section);

        // We only save the result if we are the king node
        if params.party_id == 0 {
            let (mut a, mut b, c) = (pi_a_share, pi_b_share, pi_c_share);
            // These elements are needed to construct the full proof, they are part of the proving key.
            // however, we can just send these values to the client, not the full proving key.
            a += pk.a_query[0] + pk.vk.alpha_g1;
            b += pk.b_g2_query[0] + pk.vk.beta_g2;

            let proof = ark_groth16::Proof::<E> {
                a: a.into_affine(),
                b: b.into_affine(),
                c: c.into_affine(),
            };

            tracing::info!(job_id = %job.job_id, a = %proof.a, b = %proof.b, c = %proof.c, "Proof Generated");
            // Verify the proof
            // convert the public inputs from string to bigints
            let public_inputs = job
                .public_inputs
                .iter()
                .map(|s| F::from_str(s).unwrap())
                .collect::<Vec<_>>();
            let pvk = ark_groth16::prepare_verifying_key(&pk.vk);
            let verified = Groth16::<E, CircomReduction>::verify_with_processed_vk(
                &pvk,
                public_inputs.as_slice(),
                &proof,
            )
            .unwrap();
            if verified {
                tracing::info!("Proof verified");
            } else {
                tracing::error!("Proof verification failed");
            }
            let mut proof_bytes = Vec::new();
            proof.serialize_compressed(&mut proof_bytes).unwrap();
            let proof_path = params
                .client
                .state
                .watch_dir
                .join(format!("proof_{}", job.job_id));
            tokio::fs::write(proof_path, proof_bytes).await.unwrap();
            tracing::info!("Proof written to file");
        }
        // TODO: remove the job from the queue/disk
        Ok(())
    })
}

#[derive(Clone)]
struct BlockchainClient {
    latest_received_header: Arc<AtomicU64>,
    state: State,
    tx: TracingUnboundedSender<<TestBlock as Block>::Hash>,
    #[allow(unused)]
    rx: Arc<TracingUnboundedReceiver<<TestBlock as Block>::Hash>>,
}

impl BlockchainClient {
    fn new(state: State) -> Self {
        let (tx, rx) = sc_utils::mpsc::tracing_unbounded::<<TestBlock as Block>::Hash>(
            "mpsc_finality_notification",
            999999,
        );
        Self {
            latest_received_header: Arc::new(AtomicU64::new(0)),
            state,
            tx,
            rx: Arc::new(rx),
        }
    }
}

#[async_trait::async_trait]
impl Client<TestBlock> for BlockchainClient {
    async fn get_next_finality_notification(&self) -> Option<FinalityNotification<TestBlock>> {
        self.get_latest_finality_notification().await
    }

    async fn get_latest_finality_notification(&self) -> Option<FinalityNotification<TestBlock>> {
        let _permit = self.state.processing_semophore.acquire().await.unwrap();

        let mut jobs = self.state.jobs.lock().await;
        let mut circuits = self.state.circuits.lock().await;
        let mut entries = tokio::fs::read_dir(&self.state.watch_dir).await.unwrap();

        // Fetch any new jobs from the watch directory.
        // by walking the directory and reading the job files and circuit files
        // and adding them to the state
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_str().unwrap();
            // job_{id}.json or circuit_{id}.json
            let parts = file_name.split('_').collect::<Vec<_>>();
            // job or circuit
            let prefix = parts[0];
            // if the file is not a job or circuit file, skip it
            if prefix != "job" && prefix != "circuit" {
                continue;
            }
            let id = parts[1].split('.').collect::<Vec<_>>()[0]
                .parse::<u64>()
                .unwrap();
            // convert the id to 32 bytes hash.
            let mut id_bytes = [0u8; 32];
            id_bytes[..8].copy_from_slice(&id.to_be_bytes());
            match prefix {
                "job" if jobs.contains_key(&id_bytes) => {
                    tracing::trace!("Job {} already exists", id);
                }
                "job" => {
                    // parse the job file
                    let job = tokio::fs::read_to_string(path).await.unwrap();
                    let job: JobProperties = serde_json::from_str(&job).unwrap();
                    jobs.insert(id_bytes, job);
                    tracing::debug!("Added job {} to state", id);
                }
                "circuit" if circuits.contains_key(&id) => {
                    tracing::trace!("Circuit {} already exists", id);
                }
                "circuit" => {
                    let circuit = tokio::fs::read_to_string(path).await.unwrap();
                    let circuit: CircuitProperties = serde_json::from_str(&circuit).unwrap();
                    circuits.insert(circuit.circuit_id, circuit);
                    tracing::debug!("Added circuit {} to state", id);
                }
                _ => {}
            }
        }
        let v = self.latest_received_header.fetch_add(1, Ordering::AcqRel);
        // Sleep for a while to simulate block time
        tokio::time::sleep(BLOCK_DURATION).await;
        Some(block_number_to_finality_notification(v, self.tx.clone()))
    }

    async fn get_next_block_import_notification(
        &self,
    ) -> Option<BlockImportNotification<TestBlock>> {
        future::pending().await
    }
}

#[async_trait::async_trait]
impl ClientWithApi<TestBlock> for BlockchainClient {
    async fn get_job_circuit_properties(
        &self,
        _circuit_id: u64,
    ) -> Result<Option<CircuitProperties>, webb_gadget::Error> {
        Ok(None)
    }

    async fn get_job_properties(
        &self,
        _job_id: u64,
    ) -> Result<Option<JobProperties>, webb_gadget::Error> {
        Ok(None)
    }

    async fn get_next_job(&self) -> Result<Option<JobProperties>, webb_gadget::Error> {
        let lock = self.state.jobs.lock().await;
        let maybe_job = lock.values().next().cloned();
        Ok(maybe_job)
    }
}

pub type TestBlock = sp_runtime::testing::Block<XtDummy>;
#[derive(Encode, Decode, sp_runtime::Serialize, Clone, Eq, PartialEq, Debug)]
pub struct XtDummy;

impl sp_runtime::traits::Extrinsic for XtDummy {
    type Call = ();
    type SignaturePayload = ();
}

fn block_number_to_finality_notification(
    block_number: u64,
    tx: TracingUnboundedSender<<TestBlock as Block>::Hash>,
) -> FinalityNotification<TestBlock> {
    let header = sp_runtime::generic::Header::<u64, _>::new_from_number(block_number);
    let mut slice = [0u8; 32];
    slice[..8].copy_from_slice(&block_number.to_be_bytes());
    // add random uuid to ensure uniqueness
    slice[8..24].copy_from_slice(&Uuid::new_v4().to_u128_le().to_be_bytes());

    let hash = sp_runtime::testing::H256::from(slice);
    let summary = FinalizeSummary {
        header,
        finalized: vec![hash],
        stale_heads: vec![],
    };

    FinalityNotification::<TestBlock>::from_summary(summary, tx)
}

pub fn setup_log() {
    let _ = SubscriberBuilder::default()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init();
}

struct CertBytes<'a> {
    cert: &'a [u8],
    private_key: &'a [u8],
}

impl<'a> CertToDer for CertBytes<'a> {
    fn serialize_certificate_to_der(&self) -> Result<Vec<u8>, mpc_net::MpcNetError> {
        Ok(self.cert.to_vec())
    }

    fn serialize_private_key_to_der(&self) -> Result<Vec<u8>, mpc_net::MpcNetError> {
        Ok(self.private_key.to_vec())
    }
}
