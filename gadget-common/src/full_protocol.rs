use crate::client::{
    AccountId, ClientWithApi, GadgetJobType, JobsApiForGadget, JobsClient, PalletSubmitter,
};
use crate::config::{DebugLogger, GadgetProtocol, Network, NetworkAndProtocolSetup};
use crate::gadget::message::GadgetProtocolMessage;
use crate::gadget::work_manager::WorkManager;
use crate::gadget::{JobInitMetadata, WorkManagerConfig};
use crate::keystore::{ECDSAKeyStore, KeystoreBackend};
use crate::protocol::AsyncProtocol;
use crate::Error;
use async_trait::async_trait;
use gadget_core::job::{BuiltExecutableJobWrapper, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use sc_client_api::{Backend, BlockImportNotification};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::marker::PhantomData;
use std::sync::Arc;
use tangle_primitives::roles::RoleType;
use tokio::sync::mpsc::UnboundedReceiver;

#[async_trait]
pub trait FullProtocolConfig: Clone + Send + Sync + Sized + 'static
where
    <Self::Client as ProvideRuntimeApi<Self::Block>>::Api: JobsApiForGadget<Self::Block>,
{
    type AsyncProtocolParameters: Send + Sync + Clone;
    type Client: ClientWithApi<Self::Block, Self::Backend>;
    type Block: Block;
    type Backend: Backend<Self::Block>;
    type Network: Network;
    type AdditionalNodeParameters: Clone + Send + Sync + 'static;
    type KeystoreBackend: KeystoreBackend;
    async fn new(
        client: Self::Client,
        pallet_tx: Arc<dyn PalletSubmitter>,
        network: Self::Network,
        logger: DebugLogger,
        account_id: AccountId,
    ) -> Result<Self, Error>;
    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AsyncProtocolParameters,
    ) -> Result<BuiltExecutableJobWrapper, JobError>;
    async fn initialize_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Block, Self::Backend, Self::Client>,
    ) -> Result<(), Error>;
    async fn next_message(&self) -> Option<<WorkManager as WorkManagerInterface>::ProtocolMessage>;

    async fn send_message(
        &self,
        message: <WorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), Error>;
    async fn create_next_job(
        &self,
        job: JobInitMetadata<Self::Block>,
    ) -> Result<Self::AsyncProtocolParameters, Error>;

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<Self::Block>,
        _job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn process_error(&self, error: Error, _job_manager: &ProtocolWorkManager<WorkManager>) {
        self.logger().error(format!("Error in protocol: {error}"));
    }

    fn account_id(&self) -> &AccountId;

    fn name(&self) -> String;

    fn role_filter(&self, role: RoleType) -> bool;

    fn phase_filter(&self, job: GadgetJobType) -> bool;

    fn jobs_client(&self) -> JobsClient<Self::Block, Self::Backend, Self::Client>;
    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter>;

    fn logger(&self) -> DebugLogger;

    fn client(&self) -> Self::Client;
    fn get_work_manager_config(&self) -> WorkManagerConfig {
        Default::default()
    }
}

#[async_trait]
impl<T: FullProtocolConfig> AsyncProtocol for T
where
    <T::Client as ProvideRuntimeApi<T::Block>>::Api: JobsApiForGadget<T::Block>,
{
    type AdditionalParams = T::AsyncProtocolParameters;

    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
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
impl<T: FullProtocolConfig> NetworkAndProtocolSetup for T
where
    <T::Client as ProvideRuntimeApi<T::Block>>::Api: JobsApiForGadget<T::Block>,
{
    type Network = T;
    type Protocol = T;
    type Client = T::Client;
    type Block = T::Block;
    type Backend = T::Backend;

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Block, Self::Backend, Self::Client>,
    ) -> Result<(Self::Network, Self::Protocol), Error> {
        T::initialize_network_and_protocol(self, jobs_client).await?;
        Ok((self.clone(), self.clone()))
    }

    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
        T::pallet_tx(self)
    }

    fn logger(&self) -> DebugLogger {
        T::logger(self)
    }

    fn client(&self) -> Self::Client {
        T::client(self)
    }
}

#[async_trait]
impl<T: FullProtocolConfig> GadgetProtocol<T::Block, T::Backend, T::Client> for T
where
    <T::Client as ProvideRuntimeApi<T::Block>>::Api: JobsApiForGadget<T::Block>,
{
    async fn create_next_job(
        &self,
        job: JobInitMetadata<T::Block>,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, Error> {
        T::create_next_job(self, job).await
    }

    async fn process_block_import_notification(
        &self,
        notification: BlockImportNotification<T::Block>,
        job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<(), Error> {
        T::process_block_import_notification(self, notification, job_manager).await
    }

    async fn process_error(&self, error: Error, job_manager: &ProtocolWorkManager<WorkManager>) {
        T::process_error(self, error, job_manager).await;
    }

    fn account_id(&self) -> &AccountId {
        T::account_id(self)
    }

    fn name(&self) -> String {
        T::name(self)
    }

    fn role_filter(&self, role: RoleType) -> bool {
        T::role_filter(self, role)
    }

    fn phase_filter(&self, job: GadgetJobType) -> bool {
        T::phase_filter(self, job)
    }

    fn client(&self) -> JobsClient<T::Block, T::Backend, T::Client> {
        T::jobs_client(self)
    }

    fn logger(&self) -> DebugLogger {
        T::logger(self)
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        T::get_work_manager_config(self)
    }
}

#[async_trait]
impl<T: FullProtocolConfig> Network for T
where
    <T::Client as ProvideRuntimeApi<T::Block>>::Api: JobsApiForGadget<T::Block>,
{
    async fn next_message(&self) -> Option<<WorkManager as WorkManagerInterface>::ProtocolMessage> {
        T::next_message(self).await
    }

    async fn send_message(
        &self,
        message: <WorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), Error> {
        T::send_message(self, message).await
    }
}

/// Used for constructing an instance of a node. If there is both a keygen and a signing protocol, then,
/// the length of the vectors are 2. The length of the vector is equal to the numbers of protocols that
/// the constructed node is going to concurrently execute
pub struct NodeInput<
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
    D,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    pub mock_clients: Vec<C>,
    pub mock_networks: Vec<N>,
    pub account_id: AccountId,
    pub logger: DebugLogger,
    pub pallet_tx: Arc<dyn PalletSubmitter>,
    pub keystore: ECDSAKeyStore<KBE>,
    pub node_index: usize,
    pub additional_params: D,
    pub _pd: PhantomData<(B, BE)>,
}
