use crate::client::{JobsClient, PalletSubmitter};
use crate::config::{DebugLogger, GadgetProtocol, Network, NetworkAndProtocolSetup};
use crate::environments::GadgetEnvironment;
use crate::gadget::tangle::TangleInitMetadata;
use crate::gadget::WorkManagerConfig;
use crate::keystore::{ECDSAKeyStore, KeystoreBackend};
use crate::prometheus::PrometheusConfig;
use crate::protocol::AsyncProtocol;
use crate::tangle_runtime::*;
use crate::Error;
use async_trait::async_trait;
use gadget_core::gadget::manager::AbstractGadget;
use gadget_core::job::{BuiltExecutableJobWrapper, JobError};
use gadget_core::job_manager::{ProtocolMessageMetadata, ProtocolWorkManager};
use gadget_io::tokio::sync::mpsc::UnboundedReceiver;
use parity_scale_codec::{Decode, Encode};
use parking_lot::Mutex;
use parking_lot::RwLock;
use sp_core::ecdsa::{Public, Signature};
use sp_core::{keccak_256, sr25519};
use std::sync::Arc;

pub type SharedOptional<T> = Arc<Mutex<Option<T>>>;

#[async_trait]
pub trait FullProtocolConfig<Env: GadgetEnvironment>:
    Clone + Send + Sync + Sized + 'static
{
    type AsyncProtocolParameters: Send + Sync + Clone + 'static;
    type Network: Network<Env>;
    type AdditionalNodeParameters: Clone + Send + Sync + 'static;
    type KeystoreBackend: KeystoreBackend;

    async fn new(
        client: <Env as GadgetEnvironment>::Client,
        pallet_tx: Arc<dyn PalletSubmitter>,
        network: Self::Network,
        logger: DebugLogger,
        account_id: sr25519::Public,
        key_store: ECDSAKeyStore<Self::KeystoreBackend>,
        prometheus_config: PrometheusConfig,
    ) -> Result<Self, <Env as GadgetEnvironment>::Error>;

    async fn generate_protocol_from(
        &self,
        associated_block_id: <Env as GadgetEnvironment>::Clock,
        associated_retry_id: <Env as GadgetEnvironment>::RetryID,
        associated_session_id: <Env as GadgetEnvironment>::SessionID,
        associated_task_id: <Env as GadgetEnvironment>::TaskID,
        protocol_message_rx: UnboundedReceiver<<Env as GadgetEnvironment>::ProtocolMessage>,
        additional_params: Self::AsyncProtocolParameters,
    ) -> Result<BuiltExecutableJobWrapper, JobError>;

    /// This function returns the internally-used network for the protocol.
    ///
    /// Important: This function should never be used directly, it is for internal use only.
    /// Instead, `Self` implements `Network` and should instead be used for networking, as it
    /// grants metrics and message signing and verification to messages
    fn internal_network(&self) -> &Self::Network;
    /// Given an input of metadata for a job, return a set of parameters that can be used to
    /// construct a protocol.
    ///
    /// The provided work manager should only be used for querying recorded_messages
    async fn create_next_job(
        &self,
        job: TangleInitMetadata,
        work_manager: &ProtocolWorkManager<<Env as GadgetEnvironment>::WorkManager>,
    ) -> Result<Self::AsyncProtocolParameters, <Env as GadgetEnvironment>::Error>;

    async fn process_error(
        &self,
        error: Error,
        _job_manager: &ProtocolWorkManager<<Env as GadgetEnvironment>::WorkManager>,
    ) {
        self.logger().error(format!("Error in protocol: {error}"));
    }

    fn account_id(&self) -> &sr25519::Public;

    fn name(&self) -> String;

    fn role_filter(&self, role: roles::RoleType) -> bool;

    fn phase_filter(
        &self,
        job: jobs::JobType<AccountId32, MaxParticipants, MaxSubmissionLen, MaxAdditionalParamsLen>,
    ) -> bool;

    fn jobs_client(&self) -> &SharedOptional<JobsClient<Env>>;
    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter>;

    fn logger(&self) -> DebugLogger;

    fn client(&self) -> <Env as GadgetEnvironment>::Client;
    fn get_jobs_client(&self) -> JobsClient<Env> {
        self.jobs_client()
            .lock()
            .clone()
            .expect("Jobs client not initialized")
    }
    fn key_store(&self) -> &ECDSAKeyStore<Self::KeystoreBackend>;
    fn get_work_manager_config(&self) -> WorkManagerConfig {
        Default::default()
    }
}

#[async_trait]
impl<Env: GadgetEnvironment, T: FullProtocolConfig<Env>> AsyncProtocol<Env> for T
where
    T: FullProtocolConfig<Env>,
{
    type AdditionalParams = T::AsyncProtocolParameters;

    async fn generate_protocol_from(
        &self,
        associated_block_id: <Env as GadgetEnvironment>::Clock,
        associated_retry_id: <Env as GadgetEnvironment>::RetryID,
        associated_session_id: <Env as GadgetEnvironment>::SessionID,
        associated_task_id: <Env as GadgetEnvironment>::TaskID,
        protocol_message_rx: UnboundedReceiver<<Env as GadgetEnvironment>::ProtocolMessage>,
        additional_params: <T as FullProtocolConfig<Env>>::AsyncProtocolParameters,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        T::generate_protocol_from(
            self,
            associated_block_id,
            associated_retry_id,
            associated_session_id,
            associated_task_id,
            protocol_message_rx,
            additional_params,
        )
        .await
    }
}

#[async_trait]
impl<Env, T> NetworkAndProtocolSetup<Env> for T
where
    Env: GadgetEnvironment,
    T: FullProtocolConfig<Env>,
{
    type Network = T;
    type Protocol = T;

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Env>,
    ) -> Result<(Self::Network, Self::Protocol), Error> {
        let jobs_client_store = T::jobs_client(self);
        jobs_client_store.lock().replace(jobs_client);
        Ok((self.clone(), self.clone()))
    }

    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
        T::pallet_tx(self)
    }

    fn logger(&self) -> DebugLogger {
        T::logger(self)
    }

    fn client(&self) -> <Env as GadgetEnvironment>::Client {
        T::client(self)
    }
}

#[async_trait]
impl<Env: GadgetEnvironment<Error = Error>, T: FullProtocolConfig<Env> + AsyncProtocol<Env>>
    GadgetProtocol<Env> for T
where
    T: AsyncProtocol<Env, AdditionalParams = T::AsyncProtocolParameters>,
{
    async fn create_next_job(
        &self,
        job: TangleInitMetadata,
        work_manager: &ProtocolWorkManager<Env::WorkManager>,
    ) -> Result<<Self as AsyncProtocol<Env>>::AdditionalParams, Env::Error> {
        T::create_next_job(self, job, work_manager).await
    }

    async fn process_event(
        &self,
        notification: <Env as GadgetEnvironment>::Event,
    ) -> Result<(), Env::Error> {
        T::process_event(self, notification).await
    }

    async fn process_error(
        &self,
        error: Env::Error,
        job_manager: &ProtocolWorkManager<Env::WorkManager>,
    ) {
        T::process_error(self, error, job_manager).await;
    }

    async fn generate_work_manager(
        &self,
        clock: Arc<RwLock<Option<<Env as GadgetEnvironment>::Clock>>>,
    ) -> <Env as GadgetEnvironment>::WorkManager {
        T::generate_work_manager(self, clock).await
    }

    fn account_id(&self) -> &sr25519::Public {
        T::account_id(self)
    }

    fn name(&self) -> String {
        T::name(self)
    }

    fn role_filter(&self, role: roles::RoleType) -> bool {
        T::role_filter(self, role)
    }

    fn phase_filter(
        &self,
        job: jobs::JobType<AccountId32, MaxParticipants, MaxSubmissionLen, MaxAdditionalParamsLen>,
    ) -> bool {
        T::phase_filter(self, job)
    }

    fn client(&self) -> JobsClient<Env> {
        T::get_jobs_client(self)
    }

    fn logger(&self) -> DebugLogger {
        T::logger(self)
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        T::get_work_manager_config(self)
    }
}

#[async_trait]
impl<Env: GadgetEnvironment, T: FullProtocolConfig<Env> + AbstractGadget> Network<Env> for T
where
    T: AbstractGadget<ProtocolMessage = Env::ProtocolMessage>,
{
    async fn next_message(&self) -> Option<Env::ProtocolMessage> {
        let mut message = T::internal_network(self).next_message().await?;
        let peer_public_key = message.sender_network_id();
        if let Some(peer_public_key) = peer_public_key {
            if let Ok(payload_and_signature) =
                <PayloadAndSignature as Decode>::decode(&mut message.payload().as_slice())
            {
                let hashed_message = keccak_256(&payload_and_signature.payload);
                if sp_core::ecdsa::Pair::verify_prehashed(
                    &payload_and_signature.signature,
                    &hashed_message,
                    &peer_public_key,
                ) {
                    *message.payload_mut() = payload_and_signature.payload;
                    crate::prometheus::BYTES_RECEIVED.inc_by(message.payload().len() as u64);
                    return Some(message);
                } else {
                    self.logger()
                        .warn("Received a message with an invalid signature.")
                }
            } else {
                self.logger()
                    .warn("Received a message without a valid payload and signature.")
            }
        } else {
            self.logger()
                .warn("Received a message without a valid public key from sender");
        }

        // This message was invalid. Thus, poll the next message
        self.next_message().await
    }

    async fn send_message(&self, mut message: Env::ProtocolMessage) -> Result<(), Error> {
        // Sign the hash of the message
        let hashed_message = keccak_256(message.payload());
        let signature = self.key_store().pair().sign_prehashed(&hashed_message);
        let payload_and_signature = PayloadAndSignature {
            payload: message.payload().to_vec(),
            signature,
        };

        let serialized_message = Encode::encode(&payload_and_signature);
        *message.payload_mut() = serialized_message;
        crate::prometheus::BYTES_SENT.inc_by(message.payload().len() as u64);
        T::internal_network(self).send_message(message).await
    }

    fn greatest_authority_id(&self) -> Option<Public> {
        T::internal_network(self).greatest_authority_id()
    }
}

/// Used for constructing an instance of a node. If there is both a keygen and a signing protocol, then,
/// the length of the vectors are 2. The length of the vector is equal to the numbers of protocols that
/// the constructed node is going to concurrently execute
pub struct NodeInput<Env: GadgetEnvironment, N: Network<Env>, KBE: KeystoreBackend, D> {
    pub clients: Vec<Env::Client>,
    pub networks: Vec<N>,
    pub account_id: sr25519::Public,
    pub logger: DebugLogger,
    pub pallet_tx: Arc<dyn PalletSubmitter>,
    pub keystore: ECDSAKeyStore<KBE>,
    pub node_index: usize,
    pub additional_params: D,
    pub prometheus_config: PrometheusConfig,
}

#[derive(Encode, Decode)]
pub struct PayloadAndSignature {
    payload: Vec<u8>,
    signature: Signature,
}
