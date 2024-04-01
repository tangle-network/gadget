use gadget_core::gadget::manager::GadgetError;
pub use gadget_core::job::JobError;
pub use gadget_core::job_manager::WorkManagerError;
use sp_core::ecdsa;
use std::fmt::{Debug, Display, Formatter};
use tokio::task::JoinError;

#[cfg(not(target_family = "wasm"))]
pub mod standard;

#[cfg(target_family = "wasm")]
pub mod wasm;

#[cfg(target_family = "wasm")]
pub use wasm::{
    metrics::Metrics,
    prometheus::{setup, PrometheusConfig, BYTES_RECEIVED, BYTES_SENT},
    shell::KeystoreConfig,
};

#[cfg(not(target_family = "wasm"))]
pub use standard::{
    metrics::Metrics,
    prometheus::{setup, PrometheusConfig, BYTES_RECEIVED, BYTES_SENT, REGISTRY},
    shell::KeystoreConfig,
};

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
