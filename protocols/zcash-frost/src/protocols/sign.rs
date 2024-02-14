use async_trait::async_trait;
use frame_support::BoundedVec;
use frost_core::keys::{KeyPackage, PublicKeyPackage};
use frost_ed25519::Ed25519Sha512;
use frost_ed448::Ed448Shake256;
use frost_p256::P256Sha256;
use frost_p384::P384Sha384;
use frost_ristretto255::Ristretto255Sha512;
use frost_secp256k1::Secp256K1Sha256;
use gadget_common::client::JobsApiForGadget;
use gadget_common::client::{AccountId, ClientWithApi, GadgetJobResult, GadgetJobType, JobsClient};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::{GadgetProtocol, JobInitMetadata, WorkManagerConfig};
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use gadget_common::protocol::AsyncProtocol;
use gadget_common::{Block, BlockImportNotification};
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use rand::SeedableRng;
use round_based::MpcParty;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_core::keccak_256;
use std::collections::HashMap;
use std::sync::Arc;
use tangle_primitives::jobs::{DKGTSSSignatureResult, DigitalSignatureScheme, JobId, JobType};
use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::rounds;
use crate::rounds::keygen::FrostKeyShare;

pub struct ZcashFrostSigningProtocol<B: Block, BE, KBE: KeystoreBackend, C, N> {
    client: JobsClient<B, BE, C>,
    key_store: ECDSAKeyStore<KBE>,
    network: N,
    logger: DebugLogger,
    account_id: AccountId,
}

pub async fn create_protocol<B, BE, KBE, C, N>(
    account_id: AccountId,
    client: JobsClient<B, BE, C>,
    network: N,
    logger: DebugLogger,
    key_store: ECDSAKeyStore<KBE>,
) -> ZcashFrostSigningProtocol<B, BE, KBE, C, N>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    ZcashFrostSigningProtocol {
        client,
        network,
        key_store,
        logger,
        account_id,
    }
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        KBE: KeystoreBackend,
        N: Network,
    > GadgetProtocol<B, BE, C> for ZcashFrostSigningProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    fn name(&self) -> String {
        "zcash-frost-signing".to_string()
    }

    async fn create_next_job(
        &self,
        job: JobInitMetadata<B>,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, gadget_common::Error> {
        let job_id = job.job_id;

        let JobType::DKGTSSPhaseTwo(p2_job) = job.job_type else {
            panic!("Should be valid type")
        };
        let input_data_to_sign = p2_job.submission.try_into().unwrap();
        let previous_job_id = p2_job.phase_one_id;

        let phase1_job = job.phase1_job.expect("Should exist for a phase 2 job");
        let participants = phase1_job.clone().get_participants().expect("Should exist");
        let t = phase1_job.get_threshold().expect("Should exist") as u16;

        let seed =
            keccak_256(&[&job_id.to_be_bytes()[..], &job.retry_id.to_be_bytes()[..]].concat());
        let mut rng = rand_chacha::ChaChaRng::from_seed(seed);

        let (i, signers, mapping) =
            super::util::choose_signers(&mut rng, &self.account_id, &participants, t)?;
        let key = self
            .key_store
            .get_job_result(previous_job_id)
            .await
            .map_err(|err| gadget_common::Error::ClientError {
                err: err.to_string(),
            })?
            .ok_or_else(|| gadget_common::Error::ClientError {
                err: format!("No key found for job ID: {job_id:?}"),
            })?;

        let user_id_to_account_id_mapping = Arc::new(mapping);

        let params = ZcashFrostSigningExtraParams {
            i,
            t,
            signers,
            job_id,
            role_type: job.role_type,
            keyshare: key,
            input_data_to_sign,
            user_id_to_account_id_mapping,
        };
        Ok(params)
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<(), gadget_common::Error> {
        Ok(())
    }

    async fn process_error(
        &self,
        error: gadget_common::Error,
        _job_manager: &ProtocolWorkManager<WorkManager>,
    ) {
        log::error!(target: "gadget", "Error: {error:?}");
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn role_filter(&self, role: RoleType) -> bool {
        matches!(
            role,
            RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostEd25519)
                | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostP256)
                | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostRistretto255)
                | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostSecp256k1)
        )
    }

    fn phase_filter(&self, job: GadgetJobType) -> bool {
        matches!(job, JobType::DKGTSSPhaseTwo(_))
    }

    fn client(&self) -> &JobsClient<B, BE, C> {
        &self.client
    }

    fn logger(&self) -> &DebugLogger {
        &self.logger
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: Some(crate::constants::signing_worker::JOB_POLL_INTERVAL),
            max_active_tasks: crate::constants::signing_worker::MAX_RUNNING_TASKS,
            max_pending_tasks: crate::constants::signing_worker::MAX_ENQUEUED_TASKS,
        }
    }
}

macro_rules! deserialize_and_run_threshold_sign {
    ($impl_type:ty, $keyshare:expr, $tracer:expr, $i:expr, $signers:expr, $msg:expr, $role:expr, $rng:expr, $party:expr) => {{
        let key_package =
            KeyPackage::<$impl_type>::deserialize(&$keyshare.key_package).map_err(|err| {
                JobError {
                    reason: format!("Failed to deserialize key share: {err:?}"),
                }
            })?;

        let public_key_package = PublicKeyPackage::<$impl_type>::deserialize(
            &$keyshare.pubkey_package,
        )
        .map_err(|err| JobError {
            reason: format!("Failed to deserialize public key package: {err:?}"),
        })?;

        rounds::sign::run_threshold_sign(
            Some($tracer),
            $i,
            $signers,
            (key_package, public_key_package),
            $msg,
            $role,
            $rng,
            $party,
        )
        .await
        .map_err(|err| JobError {
            reason: format!("Failed to run threshold sign: {err:?}"),
        })?
    }};
}

pub struct ZcashFrostSigningExtraParams {
    i: u16,
    t: u16,
    signers: Vec<u16>,
    job_id: JobId,
    role_type: RoleType,
    keyshare: FrostKeyShare,
    input_data_to_sign: Vec<u8>,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        KBE: KeystoreBackend,
        C: ClientWithApi<B, BE>,
        N: Network,
    > AsyncProtocol for ZcashFrostSigningProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    type AdditionalParams = ZcashFrostSigningExtraParams;
    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_channel: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let debug_logger_post = self.logger.clone();
        let logger = debug_logger_post.clone();
        let protocol_output = Arc::new(tokio::sync::Mutex::new(None));
        let protocol_output_clone = protocol_output.clone();
        let client = self.client.clone();
        let id = self.account_id;
        let network = self.network.clone();

        let (i, signers, t, keyshare, role_type, input_data_to_sign, mapping) = (
            additional_params.i,
            additional_params.signers,
            additional_params.t,
            additional_params.keyshare,
            additional_params.role_type,
            additional_params.input_data_to_sign.clone(),
            additional_params.user_id_to_account_id_mapping.clone(),
        );

        let role = match role_type {
            RoleType::Tss(role) => role,
            _ => {
                return Err(JobError {
                    reason: "Invalid role type".to_string(),
                })
            }
        };

        Ok(JobBuilder::new()
            .protocol(async move {
                let mut rng = rand::rngs::StdRng::from_entropy();
                let protocol_message_channel =
                    super::util::CloneableUnboundedReceiver::from(protocol_message_channel);

                logger.info(format!(
                    "Starting Signing Protocol with params: i={i}, t={t}"
                ));

                let (
                    signing_tx_to_outbound,
                    signing_rx_async_proto,
                    _broadcast_tx_to_outbound,
                    _broadcast_rx_from_gadget,
                ) = super::util::create_job_manager_to_async_protocol_channel_split::<_, (), _>(
                    protocol_message_channel.clone(),
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    mapping.clone(),
                    id,
                    network.clone(),
                );

                let mut tracer = dfns_cggmp21::progress::PerfProfiler::new();
                let delivery = (signing_rx_async_proto, signing_tx_to_outbound);
                let party = MpcParty::connected(delivery);
                let signature = match role {
                    ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {
                        deserialize_and_run_threshold_sign!(
                            Secp256K1Sha256,
                            keyshare,
                            &mut tracer,
                            i,
                            signers,
                            &input_data_to_sign,
                            role,
                            &mut rng,
                            party
                        )
                    }
                    ThresholdSignatureRoleType::ZcashFrostEd25519 => {
                        deserialize_and_run_threshold_sign!(
                            Ed25519Sha512,
                            keyshare,
                            &mut tracer,
                            i,
                            signers,
                            &input_data_to_sign,
                            role,
                            &mut rng,
                            party
                        )
                    }
                    ThresholdSignatureRoleType::ZcashFrostEd448 => {
                        deserialize_and_run_threshold_sign!(
                            Ed448Shake256,
                            keyshare,
                            &mut tracer,
                            i,
                            signers,
                            &input_data_to_sign,
                            role,
                            &mut rng,
                            party
                        )
                    }
                    ThresholdSignatureRoleType::ZcashFrostP256 => {
                        deserialize_and_run_threshold_sign!(
                            P256Sha256,
                            keyshare,
                            &mut tracer,
                            i,
                            signers,
                            &input_data_to_sign,
                            role,
                            &mut rng,
                            party
                        )
                    }
                    ThresholdSignatureRoleType::ZcashFrostP384 => {
                        deserialize_and_run_threshold_sign!(
                            P384Sha384,
                            keyshare,
                            &mut tracer,
                            i,
                            signers,
                            &input_data_to_sign,
                            role,
                            &mut rng,
                            party
                        )
                    }
                    ThresholdSignatureRoleType::ZcashFrostRistretto255 => {
                        deserialize_and_run_threshold_sign!(
                            Ristretto255Sha512,
                            keyshare,
                            &mut tracer,
                            i,
                            signers,
                            &input_data_to_sign,
                            role,
                            &mut rng,
                            party
                        )
                    }
                    _ => {
                        return Err(JobError {
                            reason: "Invalid role type".to_string(),
                        })
                    }
                };
                let perf_report = tracer.get_report().map_err(|err| JobError {
                    reason: format!("Signing protocol error: {err:?}"),
                })?;
                logger.trace(format!("Signing protocol report: {perf_report}"));
                logger.debug("Finished AsyncProtocol - Signing");
                *protocol_output.lock().await = Some(signature);
                Ok(())
            })
            .post(async move {
                // Submit the protocol output to the blockchain
                if let Some(signature) = protocol_output_clone.lock().await.take() {
                    // Compute the signature bytes by first converting the signature
                    // to a fixed byte array and then converting that to a Vec<u8>.
                    let (signature, signature_scheme) = match role {
                        ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {
                            let mut signature_bytes = [0u8; 64];
                            signature_bytes.copy_from_slice(&signature.group_signature);
                            (
                                signature_bytes.to_vec().try_into().unwrap(),
                                DigitalSignatureScheme::SchnorrSecp256k1,
                            )
                        }
                        ThresholdSignatureRoleType::ZcashFrostEd25519 => {
                            let mut signature_bytes = [0u8; 64];
                            signature_bytes.copy_from_slice(&signature.group_signature);
                            (
                                signature_bytes.to_vec().try_into().unwrap(),
                                DigitalSignatureScheme::SchnorrEd25519,
                            )
                        }
                        ThresholdSignatureRoleType::ZcashFrostEd448 => {
                            let mut signature_bytes = [0u8; 64];
                            signature_bytes.copy_from_slice(&signature.group_signature);
                            (
                                signature_bytes.to_vec().try_into().unwrap(),
                                DigitalSignatureScheme::SchnorrEd448,
                            )
                        }
                        ThresholdSignatureRoleType::ZcashFrostP256 => {
                            let mut signature_bytes = [0u8; 64];
                            signature_bytes.copy_from_slice(&signature.group_signature);
                            (
                                signature_bytes.to_vec().try_into().unwrap(),
                                DigitalSignatureScheme::SchnorrP256,
                            )
                        }
                        ThresholdSignatureRoleType::ZcashFrostP384 => {
                            let mut signature_bytes = [0u8; 64];
                            signature_bytes.copy_from_slice(&signature.group_signature);
                            (
                                signature_bytes.to_vec().try_into().unwrap(),
                                DigitalSignatureScheme::SchnorrP384,
                            )
                        }
                        ThresholdSignatureRoleType::ZcashFrostRistretto255 => {
                            let mut signature_bytes = [0u8; 64];
                            signature_bytes.copy_from_slice(&signature.group_signature);
                            (
                                signature_bytes.to_vec().try_into().unwrap(),
                                DigitalSignatureScheme::SchnorrRistretto255,
                            )
                        }
                        _ => {
                            return Err(JobError {
                                reason: "Invalid role type".to_string(),
                            })
                        }
                    };

                    let job_result = GadgetJobResult::DKGPhaseTwo(DKGTSSSignatureResult {
                        signature_scheme,
                        data: additional_params.input_data_to_sign.try_into().unwrap(),
                        signature,
                        verifying_key: BoundedVec::new(),
                    });

                    client
                        .submit_job_result(
                            additional_params.role_type,
                            additional_params.job_id,
                            job_result,
                        )
                        .await
                        .map_err(|err| JobError {
                            reason: format!("Failed to submit job result: {err:?}"),
                        })?;
                }

                Ok(())
            })
            .build())
    }
}
