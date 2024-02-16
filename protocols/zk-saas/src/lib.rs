use crate::network::ZkNetworkService;
use crate::protocol::ZkJobAdditionalParams;
use async_trait::async_trait;
use gadget_common::client::{
    AccountId, ClientWithApi, JobsApiForGadget, JobsClient, PalletSubmitter,
};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::full_protocol::SharedOptional;
use gadget_common::prelude::*;
use gadget_common::Error;
use mpc_net::prod::RustlsCertificate;
use network::ZkProtocolNetworkConfig;
use protocol_macros::protocol;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::sync::Arc;
use tangle_primitives::roles::ZeroKnowledgeRoleType;
use tokio_rustls::rustls::{Certificate, PrivateKey, RootCertStore};

pub mod network;
pub mod protocol;

#[protocol]
pub struct ZkProtocol<
    B: Block,
    C: ClientWithApi<B, BE>,
    BE: Backend<B>,
    N: Network,
    KBE: KeystoreBackend,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    pub network: N,
    pub pallet_tx: Arc<dyn PalletSubmitter>,
    pub client: C,
    pub key_store: ECDSAKeyStore<KBE>,
    pub jobs_client: Arc<Mutex<Option<JobsClient<B, BE, C>>>>,
    pub account_id: AccountId,
    pub logger: DebugLogger,
}

#[async_trait]
impl<
        B: Block,
        C: ClientWithApi<B, BE>,
        BE: Backend<B> + 'static,
        N: Network,
        KBE: KeystoreBackend,
    > FullProtocolConfig for ZkProtocol<B, C, BE, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    type AsyncProtocolParameters = ZkJobAdditionalParams;
    type Client = C;
    type Block = B;
    type Backend = BE;
    type Network = N;
    type AdditionalNodeParameters = ZkProtocolNetworkConfig;
    type KeystoreBackend = KBE;

    async fn new(
        client: Self::Client,
        pallet_tx: Arc<dyn PalletSubmitter>,
        network: Self::Network,
        logger: DebugLogger,
        account_id: AccountId,
        key_store: ECDSAKeyStore<Self::KeystoreBackend>,
    ) -> Result<Self, Error> {
        Ok(Self {
            network,
            pallet_tx,
            client,
            key_store,
            jobs_client: Arc::new(Mutex::new(None)),
            account_id,
            logger,
        })
    }

    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AsyncProtocolParameters,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        protocol::generate_protocol_from(
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

    fn network(&self) -> &Self::Network {
        &self.network
    }

    async fn create_next_job(
        &self,
        job: JobInitMetadata<Self::Block>,
        _work_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<Self::AsyncProtocolParameters, Error> {
        protocol::create_next_job(self, job).await
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn name(&self) -> String {
        "ZK-SAAS-PROTOCOL".to_string()
    }

    fn role_filter(&self, role: RoleType) -> bool {
        matches!(role, RoleType::ZkSaaS(ZeroKnowledgeRoleType::ZkSaaSGroth16))
    }

    fn phase_filter(&self, job: GadgetJobType) -> bool {
        matches!(job, JobType::ZkSaaSPhaseTwo(_))
    }

    fn jobs_client(&self) -> &SharedOptional<JobsClient<Self::Block, Self::Backend, Self::Client>> {
        &self.jobs_client
    }

    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
        self.pallet_tx.clone()
    }

    fn logger(&self) -> DebugLogger {
        self.logger.clone()
    }

    fn client(&self) -> Self::Client {
        self.client.clone()
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: None,
            max_active_tasks: 2,
            max_pending_tasks: 10,
        }
    }
}

pub async fn create_zk_network(config: ZkProtocolNetworkConfig) -> Result<ZkNetworkService, Error> {
    let our_identity = RustlsCertificate {
        cert: Certificate(config.public_identity_der.clone()),
        private_key: PrivateKey(config.private_identity_der.clone()),
    };

    if let Some(addr) = &config.king_bind_addr {
        ZkNetworkService::new_king(config.account_id, *addr, our_identity).await
    } else {
        let king_addr = config
            .client_only_king_addr
            .expect("King address must be specified if king bind address is not specified");

        let mut king_certs = RootCertStore::empty();
        king_certs
            .add(&Certificate(
                config
                    .client_only_king_public_identity_der
                    .clone()
                    .expect("The client must specify the identity of the king"),
            ))
            .map_err(|err| Error::InitError {
                err: err.to_string(),
            })?;

        ZkNetworkService::new_client(king_addr, config.account_id, our_identity, king_certs).await
    }
}

generate_setup_and_run_command!(ZkProtocol);
