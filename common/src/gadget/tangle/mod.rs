use crate::config::GadgetProtocol;
use crate::environments::{GadgetEnvironment, TangleEnvironment};
use crate::gadget::{EventHandler, GeneralModule};
use crate::prelude::{TangleProtocolMessage, TangleWorkManager};
use crate::tangle_runtime::AccountId32;
use crate::Network;
use async_trait::async_trait;
use gadget_core::gadget::general::Client;
use gadget_core::gadget::manager::AbstractGadget;
use gadget_core::gadget::substrate::FinalityNotification;
use gadget_core::job_manager::WorkManagerInterface;
use sp_core::ecdsa;
use std::ops::Deref;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::{jobs, roles};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_testnet_runtime::{
    MaxAdditionalParamsLen, MaxParticipants, MaxSubmissionLen,
};

pub mod runtime;

/// Endow the general module with event-handler specific code tailored towards interacting with tangle
pub struct TangleGadget<N, M> {
    module: GeneralModule<N, M, TangleEnvironment>,
}

impl<N, M> Deref for TangleGadget<N, M> {
    type Target = GeneralModule<N, M, TangleEnvironment>;

    fn deref(&self) -> &Self::Target {
        &self.module
    }
}

impl<N, M> From<GeneralModule<N, M, TangleEnvironment>> for TangleGadget<N, M> {
    fn from(module: GeneralModule<N, M, TangleEnvironment>) -> Self {
        TangleGadget { module }
    }
}

#[async_trait]
impl<
        N: Send + Sync + Network<TangleEnvironment> + 'static,
        M: GadgetProtocol<TangleEnvironment> + Send + 'static,
    > AbstractGadget for TangleGadget<N, M>
where
    Self: AbstractGadget<
        Event = <TangleEnvironment as GadgetEnvironment>::Event,
        Error = <TangleEnvironment as GadgetEnvironment>::Error,
        ProtocolMessage = <TangleEnvironment as GadgetEnvironment>::ProtocolMessage,
    >,
{
    type Event = TangleEvent;
    type ProtocolMessage = TangleProtocolMessage;
    type Error = crate::Error;

    async fn next_event(&self) -> Option<Self::Event> {
        self.protocol.client().client.next_event().await
    }

    async fn get_next_protocol_message(&self) -> Option<Self::ProtocolMessage> {
        self.network.next_message().await
    }

    async fn on_event_received(&self, notification: Self::Event) -> Result<(), Self::Error> {
        self.process_event(notification).await
    }

    async fn process_protocol_message(
        &self,
        message: Self::ProtocolMessage,
    ) -> Result<(), Self::Error> {
        self.job_manager
            .deliver_message(message)
            .map(|_| ())
            .map_err(|err| crate::Error::WorkManagerError { err })
    }

    async fn process_error(&self, _error: Self::Error) {}
}

pub type TangleEvent = FinalityNotification;
pub type TangleNetworkMessage = TangleProtocolMessage;

#[async_trait]
impl<N: Send + Sync + 'static, M: GadgetProtocol<TangleEnvironment> + Send + 'static>
    EventHandler<TangleEnvironment> for TangleGadget<N, M>
where
    Self: AbstractGadget<
        Event = TangleEvent,
        ProtocolMessage = TangleNetworkMessage,
        Error = crate::Error,
    >,
{
    async fn process_event(&self, _notification: TangleEvent) -> Result<(), crate::Error> {
        // TODO: Add v2 version of the v1 code found in the commit below
        // See commit 747b3cb3d7d5418cb37ecda02e6e79d6d5343adf for v1 code
        Ok(())
    }
}

pub struct JobInitMetadata {
    pub job_type:
        jobs::JobType<AccountId32, MaxParticipants, MaxSubmissionLen, MaxAdditionalParamsLen>,
    pub role_type: roles::RoleType,
    /// This value only exists if this is a stage2 job
    pub phase1_job: Option<
        jobs::JobType<AccountId32, MaxParticipants, MaxSubmissionLen, MaxAdditionalParamsLen>,
    >,
    pub participants_role_ids: Vec<ecdsa::Public>,
    pub task_id: <TangleWorkManager as WorkManagerInterface>::TaskID,
    pub retry_id: <TangleWorkManager as WorkManagerInterface>::RetryID,
    pub job_id: u64,
    pub now: <TangleWorkManager as WorkManagerInterface>::Clock,
    pub at: [u8; 32],
}
