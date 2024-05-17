use crate::client::ClientWithApi;
use crate::config::ProtocolConfig;
use crate::gadget::work_manager::WorkManager;
use crate::gadget::{GadgetProtocol, Module};
use crate::prelude::PrometheusConfig;
use gadget::network::Network;
use gadget_core::gadget::manager::{AbstractGadget, GadgetError, GadgetManager};
use gadget_core::gadget::substrate::{Client, FinalityNotification, SubstrateGadget};
pub use gadget_core::job::JobError;
pub use gadget_core::job::*;
pub use gadget_core::job_manager::WorkManagerInterface;
pub use gadget_core::job_manager::{PollMethod, ProtocolWorkManager, WorkManagerError};
use gadget_io::tokio::task::JoinError;
use parking_lot::RwLock;
pub use sp_core;
use sp_core::ecdsa;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

pub use subxt_signer;
pub use tangle_subxt;

#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    pub use crate::client::*;
    pub use crate::config::*;
    pub use crate::full_protocol::{FullProtocolConfig, NodeInput};
    pub use crate::gadget::message::GadgetProtocolMessage;
    pub use crate::gadget::work_manager::WorkManager;
    pub use crate::gadget::JobInitMetadata;
    pub use crate::gadget::WorkManagerConfig;
    pub use crate::generate_setup_and_run_command;
    pub use crate::keystore::{ECDSAKeyStore, InMemoryBackend, KeystoreBackend};
    pub use crate::{BuiltExecutableJobWrapper, JobBuilder, JobError, WorkManagerInterface};
    pub use async_trait::async_trait;
    pub use gadget_core::job_manager::ProtocolWorkManager;
    pub use gadget_core::job_manager::SendFuture;
    pub use gadget_core::job_manager::WorkManagerError;
    pub use parking_lot::Mutex;
    pub use protocol_macros::protocol;
    pub use sp_runtime::traits::Block;
    pub use std::pin::Pin;
    pub use std::sync::Arc;
    pub use gadget_io;
    pub use gadget_io::tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
}

#[cfg(feature = "tangle-testnet")]
pub mod tangle_runtime {
    pub use tangle_subxt::subxt::utils::AccountId32;
    pub use tangle_subxt::tangle_testnet_runtime::api;
    pub use tangle_subxt::tangle_testnet_runtime::api::runtime_types::{
        bounded_collections::bounded_vec::BoundedVec,
        tangle_primitives::jobs::{
            self,
            tss::{self, *},
            zksaas::{self, *},
            JobType::*,
            PhaseResult, RpcResponseJobsData,
        },
        tangle_primitives::roles::{self, RoleType},
        tangle_testnet_runtime::{
            MaxAdditionalParamsLen, MaxDataLen, MaxKeyLen, MaxParticipants, MaxProofLen,
            MaxSignatureLen, MaxSubmissionLen,
        },
    };
}

#[cfg(feature = "tangle-mainnet")]
pub mod tangle_runtime {
    pub use tangle_subxt::subxt::utils::AccountId32;
    pub use tangle_subxt::tangle_mainnet_runtime::api;
    pub use tangle_subxt::tangle_mainnet_runtime::api::runtime_types::{
        bounded_collections::bounded_vec::BoundedVec,
        tangle_primitives::jobs::{
            self,
            tss::{self, *},
            zksaas::{self, *},
            JobType::*,
            PhaseResult,
        },
        tangle_primitives::roles::{self, RoleType},
        tangle_runtime::{
            MaxAdditionalParamsLen, MaxDataLen, MaxKeyLen, MaxParticipants, MaxProofLen,
            MaxSignatureLen, MaxSubmissionLen,
        },
    };
}

pub mod channels;
pub mod client;
pub mod config;
pub mod debug_logger;
pub mod full_protocol;
pub mod gadget;
pub mod helpers;
pub mod keystore;
pub mod locks;
pub mod prometheus;
pub mod protocol;
pub mod tracer;
pub mod utils;

#[derive(Debug)]
pub enum Error {
    RegistryCreateError { err: String },
    RegistrySendError { err: String },
    RegistryRecvError { err: String },
    RegistrySerializationError { err: String },
    RegistryListenError { err: String },
    GadgetManagerError { err: GadgetError },
    InitError { err: String },
    WorkManagerError { err: WorkManagerError },
    ProtocolRemoteError { err: String },
    ClientError { err: String },
    JobError { err: JobError },
    NetworkError { err: String },
    KeystoreError { err: String },
    MissingNetworkId,
    PeerNotFound { id: ecdsa::Public },
    JoinError { err: JoinError },
    ParticipantNotSelected { id: ecdsa::Public, reason: String },
    PrometheusError { err: String },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

impl From<JobError> for Error {
    fn from(err: JobError) -> Self {
        Error::JobError { err }
    }
}

pub async fn run_protocol<T: ProtocolConfig>(mut protocol_config: T) -> Result<(), Error> {
    gadget_io::log(&format!("PROTOCOL NOW RUNNING"));
    let client = protocol_config.take_client();
    let network = protocol_config.take_network();
    let protocol = protocol_config.take_protocol();

    let prometheus_config = protocol_config.prometheus_config();

    gadget_io::log(&format!("PROTOCOL - LATEST FINALITY NOTIFICATION"));
    // Before running, wait for the first finality notification we receive
    let latest_finality_notification =
        get_latest_finality_notification_from_client(&client).await?;
    gadget_io::log(&format!("PROTOCOL - CREATING WORK MANAGER"));
    let work_manager = create_work_manager(&latest_finality_notification, &protocol).await?;
    gadget_io::log(&format!("PROTOCOL - CREATING PROTOCOL MODULE"));
    let proto_module = Module::new(network.clone(), protocol, work_manager);
    // Plug the module into the substrate gadget to interface the WebbGadget with Substrate
    gadget_io::log(&format!("PROTOCOL - CREATING SUBSTRATE GADGET"));
    let substrate_gadget = SubstrateGadget::new(client, proto_module);
    gadget_io::log(&format!("PROTOCOL - CREATING NETWORK FUTURE"));
    let network_future = network.run();
    gadget_io::log(&format!("PROTOCOL - CREATING GADGET FUTURE"));
    let gadget_future = async move {
        // Poll the first finality notification to ensure clients can execute without having to wait
        // for another block to be produced
        if let Err(err) = substrate_gadget
            .process_finality_notification(latest_finality_notification)
            .await
        {
            substrate_gadget.process_error(err).await;
        }

        GadgetManager::new(substrate_gadget)
            .await
            .map_err(|err| Error::GadgetManagerError { err })
    };

    gadget_io::log(&format!("PROTOCOL - SETTING UP PROMETHEUS CONFIG"));
    if let Err(err) = prometheus::setup(prometheus_config.clone()).await {
        gadget_io::log(&format!("PROTOCOL - PROMETHEUS CONFIG ERROR: {:?}", err));
        protocol_config
            .logger()
            .warn(format!("Error setting up prometheus: {err:?}"));
    } else if let PrometheusConfig::Enabled { bind_addr } = prometheus_config {
        gadget_io::log(&format!("PROTOCOL - PROMETHEUS CONFIG ENABLED: {bind_addr}"));
        protocol_config
            .logger()
            .info(format!("Prometheus enabled on {bind_addr}"));
    }

    gadget_io::log(&format!("PROTOCOL - JOINING NETWORK AND GADGET FUTURES"));
    // Run both the network and the gadget together
    gadget_io::tokio::try_join!(network_future, gadget_future).map(|_| ())
}

/// Creates a work manager
pub async fn create_work_manager<C: ClientWithApi, P: GadgetProtocol<C>>(
    latest_finality_notification: &FinalityNotification,
    protocol: &P,
) -> Result<ProtocolWorkManager<WorkManager>, Error> {
    gadget_io::log(&format!("WORK MANAGER - STARTING"));
    let now: u64 = latest_finality_notification.number;

    gadget_io::log(&format!("WORK MANAGER - GETTING WORK MANAGER CONFIG"));
    let work_manager_config = protocol.get_work_manager_config();

    gadget_io::log(&format!("WORK MANAGER - CREATING CLOCK"));
    let clock = Arc::new(RwLock::new(Some(now)));

    gadget_io::log(&format!("WORK MANAGER - CREATING ZK JOB MANAGER"));
    let job_manager_zk = WorkManager {
        clock,
        logger: protocol.logger().clone(),
    };

    gadget_io::log(&format!("WORK MANAGER - SETTING POLL METHOD"));
    let poll_method = match work_manager_config.interval {
        Some(interval) => PollMethod::Interval {
            millis: interval.as_millis() as u64,
        },
        None => PollMethod::Manual,
    };

    gadget_io::log(&format!("WORK MANAGER - RETURNING PROTOCOLWORKMANAGER"));
    Ok(ProtocolWorkManager::new(
        job_manager_zk,
        work_manager_config.max_active_tasks,
        work_manager_config.max_pending_tasks,
        poll_method,
    ))
}

async fn get_latest_finality_notification_from_client<C: Client>(
    client: &C,
) -> Result<FinalityNotification, Error> {
    client
        .get_latest_finality_notification()
        .await
        .ok_or_else(|| Error::InitError {
            err: "No finality notification received".to_string(),
        })
}

#[macro_export]
/// Generates a run function that returns a future that runs all the supplied protocols run concurrently
/// Also generates a setup_node function that sets up the future that runs all the protocols concurrently
#[allow(clippy::crate_in_macro_def)]
macro_rules! generate_setup_and_run_command {
    ($( $config:ident ),*) => {
        /// Sets up a future that runs all the protocols concurrently
        pub fn setup_node<C: ClientWithApi + 'static, N: Network, KBE: $crate::keystore::KeystoreBackend, D: Send + Clone + 'static>(node_input: NodeInput<C, N, KBE, D>) -> impl SendFuture<'static, ()>
        {
            async move {
                gadget_io::log(&format!("SETUP NODE"));
                if let Err(err) = run(
                    node_input.clients,
                    node_input.pallet_tx,
                    node_input.networks,
                    node_input.logger.clone(),
                    node_input.account_id,
                    node_input.keystore,
                    node_input.prometheus_config,
                )
                .await
                {
                gadget_io::log(&format!("ERROR RUNNING GADGET: {:?}", err));
                    node_input
                        .logger
                        .error(format!("Error running gadget: {:?}", err));
                }
                gadget_io::log(&format!("SETUP NODE SUCCESSFUL"));
            }
        }

        pub async fn run<C: ClientWithApi + 'static, N: Network, KBE: $crate::keystore::KeystoreBackend>(
            client: Vec<C>,
            pallet_tx: Arc<dyn PalletSubmitter>,
            networks: Vec<N>,
            logger: DebugLogger,
            account_id: sp_core::sr25519::Public,
            key_store: ECDSAKeyStore<KBE>,
            prometheus_config: $crate::prometheus::PrometheusConfig,
        ) -> Result<(), Error>
        {
            gadget_io::log(&format!("ENTERING RUN FUNCTION"));
            use futures::TryStreamExt;
            let futures = futures::stream::FuturesUnordered::new();
            let mut networks: std::collections::VecDeque<_> = networks.into_iter().collect();
            let mut clients: std::collections::VecDeque<_> = client.into_iter().collect();

            gadget_io::log(&format!("RUN MACRO START"));
            $(
                let config = crate::$config::new(clients.pop_front().expect("Not enough clients"), pallet_tx.clone(), networks.pop_front().expect("Not enough networks"), logger.clone(), account_id.clone(), key_store.clone(), prometheus_config.clone()).await?;
                gadget_io::log(&format!("PUSHING PROTOCOL FUTURE"));
                futures.push(Box::pin(config.execute()) as std::pin::Pin<Box<dyn SendFuture<'static, Result<(), $crate::Error>>>>);
            )*
            gadget_io::log(&format!("RUN MACRO END"));

            if let Err(err) = futures.try_collect::<Vec<_>>().await.map(|_| ()) {
                gadget_io::log(&format!("ERROR COLLECTING RUN FUTURES: {:?}", err));
                Err(err)
            } else {
                gadget_io::log(&format!("SUCCESSFULLY COLLECTED RUN FUTURES"));
                Ok(())
            }
        }
    };
}

#[macro_export]
macro_rules! generate_protocol {
    ($name:expr, $struct_name:ident, $async_proto_params:ty, $proto_gen_path:expr, $create_job_path:expr, $phase_filter:pat, $( $role_filter:pat ),*) => {
        #[protocol]
        pub struct $struct_name<
            C: ClientWithApi + 'static,
            N: Network,
            KBE: KeystoreBackend,
        > {
            pallet_tx: Arc<dyn PalletSubmitter>,
            logger: DebugLogger,
            client: C,
            /// This field should NEVER be used directly. Use Self instead as the network
            network_inner: N,
            account_id: sp_core::sr25519::Public,
            key_store: ECDSAKeyStore<KBE>,
            jobs_client: Arc<Mutex<Option<JobsClient<C>>>>,
            prometheus_config: $crate::prometheus::PrometheusConfig,
        }

        #[async_trait]
        impl<
                C: ClientWithApi + 'static,
                N: Network,
                KBE: KeystoreBackend,
            > FullProtocolConfig for $struct_name<C, N, KBE>
        {
            type AsyncProtocolParameters = $async_proto_params;
            type Client = C;
            type Network = N;
            type AdditionalNodeParameters = ();
            type KeystoreBackend = KBE;

            async fn new(
                client: Self::Client,
                pallet_tx: Arc<dyn PalletSubmitter>,
                network_inner: Self::Network,
                logger: DebugLogger,
                account_id: sp_core::sr25519::Public,
                key_store: ECDSAKeyStore<Self::KeystoreBackend>,
                prometheus_config: $crate::prometheus::PrometheusConfig,
            ) -> Result<Self, Error> {
                gadget_io::log(&format!("NEW PROTOCOL {}", stringify!($name)));
                let logger = if logger.peer_id.is_empty() {
                    DebugLogger { peer_id: stringify!($name).replace("\"", "").into() }
                } else {
                    DebugLogger { peer_id: (logger.peer_id + " | " + stringify!($name)).replace("\"", "") }
                };
                gadget_io::log(&format!("NEW - LOGGER SUCCESSFULLY SET -> RETURNING"));
                Ok(Self {
                    pallet_tx,
                    logger,
                    client,
                    network_inner,
                    account_id,
                    key_store,
                    prometheus_config,
                    jobs_client: Arc::new(parking_lot::Mutex::new(None)),
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
                $proto_gen_path(
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

            fn internal_network(&self) -> &Self::Network {
                &self.network_inner
            }

            async fn create_next_job(
                &self,
                job: JobInitMetadata,
                work_manager: &ProtocolWorkManager<WorkManager>,
            ) -> Result<Self::AsyncProtocolParameters, Error> {
                $create_job_path(self, job, work_manager).await
            }

            fn account_id(&self) -> &sp_core::sr25519::Public {
                &self.account_id
            }

            fn name(&self) -> String {
                $name.to_string()
            }

            fn role_filter(&self, role: roles::RoleType) -> bool {
                $(
                    if matches!(role, $role_filter) {
                        return true;
                    }
                )*

                false
            }

            fn phase_filter(
                &self,
                job: jobs::JobType<AccountId32, MaxParticipants, MaxSubmissionLen, MaxAdditionalParamsLen>,
            ) -> bool {
                matches!(job, $phase_filter)
            }

            fn jobs_client(&self) -> &SharedOptional<JobsClient<Self::Client>> {
                &self.jobs_client
            }

            fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
                self.pallet_tx.clone()
            }

            fn logger(&self) -> DebugLogger {
                self.logger.clone()
            }

            fn key_store(&self) -> &ECDSAKeyStore<Self::KeystoreBackend> {
                &self.key_store
            }

            fn client(&self) -> Self::Client {
                self.client.clone()
            }
        }
    };
}
