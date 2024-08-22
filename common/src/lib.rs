#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::fmt::{Debug, Display, Formatter};
use alloc::string::String;

use gadget_core::gadget::error::GadgetError;
#[cfg(feature = "std")]
pub use gadget_core::job::manager::{ProtocolWorkManager, WorkManagerError};
pub use gadget_core::job::{
    error::JobError,
    protocol::{PollMethod, WorkManagerInterface},
};

use sp_core::ecdsa;

cfg_if::cfg_if! {
    if #[cfg(all(feature = "std", feature = "substrate"))] {
        use crate::config::ProtocolConfig;
        use crate::prelude::PrometheusConfig;
        use crate::module::network::Network;
        use crate::module::{GeneralModule, GadgetProtocol};
        use crate::environments::{EventMetadata, GadgetEnvironment};
        use gadget_core::gadget::general::GeneralGadget;
        use gadget_core::gadget::AbstractGadget;
        use gadget_core::gadget::manager::GadgetManager;
        use gadget_core::gadget::general::Client;
        use parking_lot::RwLock;
        use std::sync::Arc;

        pub mod module;
        pub mod config;
        pub mod full_protocol;
        pub mod protocol;
    }
}

pub mod environments;

#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    #[cfg(feature = "std")]
    pub use crate::client::*;
    #[cfg(all(feature = "std", feature = "substrate"))]
    pub use crate::config::*;
    pub use crate::environments::*;
    #[cfg(all(feature = "std", feature = "substrate"))]
    pub use crate::full_protocol::{FullProtocolConfig, NodeInput};
    pub use crate::generate_setup_and_run_command;
    #[cfg(feature = "std")]
    pub use crate::keystore::InMemoryBackend;
    pub use crate::keystore::{ECDSAKeyStore, KeystoreBackend};
    #[cfg(all(feature = "std", feature = "substrate"))]
    pub use crate::module::WorkManagerConfig;
    pub use alloc::sync::Arc;
    pub use async_trait::async_trait;
    pub use core::pin::Pin;
    pub use gadget_core::job::builder::{BuiltExecutableJobWrapper, JobBuilder};
    pub use gadget_core::job::error::JobError;
    #[cfg(feature = "std")]
    pub use gadget_core::job::manager::{ProtocolWorkManager, WorkManagerError};
    pub use gadget_core::job::protocol::WorkManagerInterface;
    pub use gadget_core::job::SendFuture;
    #[cfg(feature = "std")]
    pub use gadget_io::tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
    pub use parking_lot::Mutex;
    pub use sp_runtime::traits::Block;
}

// Convenience re-exports
pub use async_trait;
pub use gadget_io;
#[cfg(feature = "substrate")]
pub use subxt_signer;
#[cfg(feature = "substrate")]
pub use tangle_subxt;

#[cfg(feature = "substrate")]
pub mod tangle_runtime {
    pub use tangle_subxt::subxt::utils::AccountId32;
    pub use tangle_subxt::tangle_testnet_runtime::api;
    pub use tangle_subxt::tangle_testnet_runtime::api::runtime_types::{
        bounded_collections::bounded_vec::BoundedVec,
    };
}

pub mod channels;
#[cfg(feature = "std")]
pub mod client;
pub mod debug_logger;
#[cfg(feature = "std")]
pub mod helpers;
pub mod keystore;
#[cfg(feature = "std")]
pub mod locks;
#[cfg(feature = "substrate")]
pub mod prometheus;
#[cfg(feature = "std")]
pub mod tracer;
pub mod utils;

#[derive(Debug)]
pub enum Error {
    RegistryCreateError {
        err: String,
    },
    RegistrySendError {
        err: String,
    },
    RegistryRecvError {
        err: String,
    },
    RegistrySerializationError {
        err: String,
    },
    RegistryListenError {
        err: String,
    },
    GadgetManagerError {
        err: GadgetError,
    },
    InitError {
        err: String,
    },
    #[cfg(feature = "std")]
    WorkManagerError {
        err: WorkManagerError,
    },
    ProtocolRemoteError {
        err: String,
    },
    ClientError {
        err: String,
    },
    DispatchError {
        err: String,
    },
    JobError {
        err: JobError,
    },
    NetworkError {
        err: String,
    },
    KeystoreError {
        err: String,
    },
    MissingNetworkId,
    PeerNotFound {
        id: ecdsa::Public,
    },
    #[cfg(feature = "std")]
    JoinError {
        err: gadget_io::tokio::task::JoinError,
    },
    ParticipantNotSelected {
        id: ecdsa::Public,
        reason: String,
    },
    PrometheusError {
        err: String,
    },
    #[cfg(feature = "substrate")]
    Subxt {
        err: subxt::Error,
    },
    Other {
        err: String,
    },
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self::Other { err }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl core::error::Error for Error {}

impl From<JobError> for Error {
    fn from(err: JobError) -> Self {
        Error::JobError { err }
    }
}

#[cfg(feature = "substrate")]
impl From<subxt::Error> for Error {
    fn from(err: subxt::Error) -> Self {
        Error::Subxt { err }
    }
}

#[cfg(all(feature = "std", feature = "substrate"))]
pub async fn run_protocol<Env: GadgetEnvironment, T: ProtocolConfig<Env>>(
    mut protocol_config: T,
) -> Result<(), Error> {
    let client = protocol_config.take_client();
    let network = protocol_config.take_network();
    let protocol = protocol_config.take_protocol();

    let prometheus_config = protocol_config.prometheus_config();

    // Before running, wait for the first finality notification we receive
    let latest_finality_notification = get_latest_event_from_client::<Env>(&client).await?;
    let work_manager = create_work_manager(&latest_finality_notification, &protocol).await?;
    let proto_module = GeneralModule::new(network.clone(), protocol, work_manager);
    // Plug the module into the general gadget to interface the WebbGadget with Substrate
    let substrate_gadget = GeneralGadget::new(client, proto_module);
    let network_future = network.run();
    let gadget_future = async move {
        // Poll the first finality notification to ensure clients can execute without having to wait
        // for another block to be produced
        if let Err(err) = substrate_gadget
            .on_event_received(latest_finality_notification)
            .await
        {
            substrate_gadget.process_error(err).await;
        }

        GadgetManager::new(substrate_gadget)
            .await
            .map_err(|err| Error::GadgetManagerError { err })
    };

    if let Err(err) = prometheus::setup(prometheus_config.clone()).await {
        protocol_config
            .logger()
            .warn(format!("Error setting up prometheus: {err:?}"));
    } else if let PrometheusConfig::Enabled { bind_addr } = prometheus_config {
        protocol_config
            .logger()
            .info(format!("Prometheus enabled on {bind_addr}"));
    }

    // Run both the network and the gadget together
    gadget_io::tokio::try_join!(network_future, gadget_future).map(|_| ())
}

#[cfg(all(feature = "std", feature = "substrate"))]
/// Creates a work manager
pub async fn create_work_manager<Env: GadgetEnvironment, P: GadgetProtocol<Env>>(
    latest_event: &<Env as GadgetEnvironment>::Event,
    protocol: &P,
) -> Result<ProtocolWorkManager<<Env as GadgetEnvironment>::WorkManager>, Error> {
    let now = latest_event.number();

    let work_manager_config = protocol.get_work_manager_config();

    let clock = Arc::new(RwLock::new(Some(now)));

    let job_manager = protocol.generate_work_manager(clock.clone()).await;

    let poll_method = match work_manager_config.interval {
        Some(interval) => PollMethod::Interval {
            millis: interval.as_millis() as u64,
        },
        None => PollMethod::Manual,
    };

    Ok(ProtocolWorkManager::new(
        job_manager,
        work_manager_config.max_active_tasks,
        work_manager_config.max_pending_tasks,
        poll_method,
    ))
}

#[cfg(all(feature = "std", feature = "substrate"))]
async fn get_latest_event_from_client<Env: GadgetEnvironment>(
    client: &<Env as GadgetEnvironment>::Client,
) -> Result<<Env as GadgetEnvironment>::Event, Error> {
    Client::<Env::Event>::latest_event(client)
        .await
        .ok_or_else(|| Error::InitError {
            err: "No event received".to_string(),
        })
}

#[macro_export]
/// Generates a run function that returns a future that runs all the supplied protocols run concurrently
/// Also generates a setup_node function that sets up the future that runs all the protocols concurrently
#[allow(clippy::crate_in_macro_def)]
macro_rules! generate_setup_and_run_command {
    ($( $config:ident ),*) => {
        /// Sets up a future that runs all the protocols concurrently
        pub fn setup_node<Env: GadgetEnvironment, N: Network<Env>, KBE: $crate::keystore::KeystoreBackend, D: Send + Clone + 'static>(node_input: NodeInput<Env, N, KBE, D>) -> impl SendFuture<'static, ()>
        {
            async move {
                if let Err(err) = run(
                    node_input.clients,
                    node_input.tx_manager,
                    node_input.networks,
                    node_input.logger.clone(),
                    node_input.account_id,
                    node_input.keystore,
                    node_input.prometheus_config,
                )
                .await
                {
                    node_input
                        .logger
                        .error(format!("Error running gadget: {:?}", err));
                }
            }
        }

        pub async fn run<Env: GadgetEnvironment, N: Network<Env>, KBE: $crate::keystore::KeystoreBackend>(
            client: Vec<Env::Client>,
            tx_manager: <Env as GadgetEnvironment>::TransactionManager,
            networks: Vec<N>,
            logger: DebugLogger,
            account_id: sp_core::sr25519::Public,
            key_store: ECDSAKeyStore<KBE>,
            prometheus_config: $crate::prometheus::PrometheusConfig,
        ) -> Result<(), Error>
        {
            use futures::TryStreamExt;
            let futures = futures::stream::FuturesUnordered::new();
            let mut networks: std::collections::VecDeque<_> = networks.into_iter().collect();
            let mut clients: std::collections::VecDeque<_> = client.into_iter().collect();

            $(
                let config = crate::$config::new(clients.pop_front().expect("Not enough clients"), tx_manager.clone(), networks.pop_front().expect("Not enough networks"), logger.clone(), account_id.clone(), key_store.clone(), prometheus_config.clone()).await?;
                futures.push(Box::pin(config.execute()) as std::pin::Pin<Box<dyn SendFuture<'static, Result<(), $crate::Error>>>>);
            )*

            if let Err(err) = futures.try_collect::<Vec<_>>().await.map(|_| ()) {
                Err(err)
            } else {
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
            Env: GadgetEnvironment,
            N: Network<Env>,
            KBE: KeystoreBackend,
        > {
            tx_manager: <Env as GadgetEnvironment>::TransactionManager,
            logger: DebugLogger,
            client: <Env as GadgetEnvironment>::Client,
            /// This field should NEVER be used directly. Use Self instead as the network
            network_inner: N,
            account_id: sp_core::sr25519::Public,
            key_store: ECDSAKeyStore<KBE>,
            jobs_client: Arc<Mutex<Option<JobsClient<Env>>>>,
            prometheus_config: $crate::prometheus::PrometheusConfig,
        }

        #[async_trait]
        impl<
                Env: GadgetEnvironment,
                N: Network<Env>,
                KBE: KeystoreBackend,
            > FullProtocolConfig<Env> for $struct_name<Env, N, KBE>
        {
            type AsyncProtocolParameters = $async_proto_params;
            type Network = N;
            type AdditionalNodeParameters = ();
            type KeystoreBackend = KBE;

            async fn new(
                client: <Env as GadgetEnvironment>::Client,
                tx_manager: <Env as GadgetEnvironment>::TransactionManager,
                network_inner: Self::Network,
                logger: DebugLogger,
                account_id: sp_core::sr25519::Public,
                key_store: ECDSAKeyStore<Self::KeystoreBackend>,
                prometheus_config: $crate::prometheus::PrometheusConfig,
            ) -> Result<Self, Error> {
                let logger = if logger.id.is_empty() {
                    DebugLogger { id: stringify!($name).replace("\"", "").into() }
                } else {
                    DebugLogger { id: (logger.id + " | " + stringify!($name)).replace("\"", "") }
                };
                Ok(Self {
                    tx_manager,
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
                associated_block_id: <<Env as GadgetEnvironment>::WorkManager as WorkManagerInterface>::Clock,
                associated_retry_id: <<Env as GadgetEnvironment>::WorkManager as WorkManagerInterface>::RetryID,
                associated_session_id: <<Env as GadgetEnvironment>::WorkManager as WorkManagerInterface>::SessionID,
                associated_task_id: <<Env as GadgetEnvironment>::WorkManager as WorkManagerInterface>::TaskID,
                protocol_message_rx: UnboundedReceiver<<Env as GadgetEnvironment>::ProtocolMessage>,
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
                work_manager: &ProtocolWorkManager<<Env as GadgetEnvironment>::WorkManager>,
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

            fn jobs_client(&self) -> &SharedOptional<JobsClient<Env>> {
                &self.jobs_client
            }

            fn tx_manager(&self) -> <Env as GadgetEnvironment>::TransactionManager {
                self.tx_manager.clone()
            }

            fn logger(&self) -> DebugLogger {
                self.logger.clone()
            }

            fn key_store(&self) -> &ECDSAKeyStore<Self::KeystoreBackend> {
                &self.key_store
            }

            fn client(&self) -> <Env as GadgetEnvironment>::Client {
                self.client.clone()
            }
        }
    };
}
