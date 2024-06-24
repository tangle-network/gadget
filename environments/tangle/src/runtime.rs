use std::sync::Arc;
use std::time::Duration;

use crate::api::ClientWithServicesApi;
use crate::gadget::TangleEvent;
use crate::TangleEnvironment;
use gadget_common::locks::TokioMutexExt;
use gadget_common::tangle_subxt::subxt::blocks::{Block, BlockRef};
use gadget_common::tangle_subxt::subxt::ext::futures::TryFutureExt;
use gadget_common::tangle_subxt::subxt::{self, PolkadotConfig};
use gadget_common::tangle_subxt::tangle_testnet_runtime::api;
use gadget_common::{async_trait, tangle_runtime::*};
use gadget_common::tangle_subxt::tangle_testnet_runtime::api::services::storage::types::blueprints::Blueprints;
use gadget_core::gadget::general::Client;
use gadget_core::gadget::substrate::FinalityNotification;

pub type TangleConfig = subxt::PolkadotConfig;
type TangleClient = subxt::OnlineClient<TangleConfig>;
type TangleBlock = Block<TangleConfig, TangleClient>;
type TangleBlockStream = subxt::backend::StreamOfResults<TangleBlock>;

pub mod crypto {
    use sp_application_crypto::{app_crypto, ecdsa, sr25519};
    pub mod acco {
        use super::*;
        pub use sp_core::crypto::key_types::ACCOUNT as KEY_TYPE;
        app_crypto!(sr25519, KEY_TYPE);
    }

    pub mod role {
        use super::*;
        /// Key type for ROLE keys
        pub const KEY_TYPE: sp_application_crypto::KeyTypeId =
            sp_application_crypto::KeyTypeId(*b"role");

        app_crypto!(ecdsa, KEY_TYPE);
    }
}

#[derive(Debug, Clone)]
pub struct TangleRuntime {
    client: subxt::OnlineClient<PolkadotConfig>,
    finality_notification_stream:
        Arc<gadget_common::gadget_io::tokio::sync::Mutex<Option<TangleBlockStream>>>,
    latest_finality_notification:
        Arc<gadget_common::gadget_io::tokio::sync::Mutex<Option<FinalityNotification>>>,
}

impl TangleRuntime {
    /// Create a new TangleRuntime instance.
    pub fn new(client: subxt::OnlineClient<PolkadotConfig>) -> Self {
        Self {
            client,
            finality_notification_stream: Arc::new(
                gadget_common::gadget_io::tokio::sync::Mutex::new(None),
            ),
            latest_finality_notification: Arc::new(
                gadget_common::gadget_io::tokio::sync::Mutex::new(None),
            ),
        }
    }

    pub fn client(&self) -> subxt::OnlineClient<PolkadotConfig> {
        self.client.clone()
    }

    /// Initialize the TangleRuntime instance by listening for finality notifications.
    /// This method must be called before using the instance.
    async fn initialize(&self) -> gadget_common::color_eyre::Result<()> {
        let finality_notification_stream = self.client.blocks().subscribe_finalized().await?;
        *self.finality_notification_stream.lock().await = Some(finality_notification_stream);
        Ok(())
    }

    fn runtime_api(
        &self,
        at: [u8; 32],
    ) -> subxt::runtime_api::RuntimeApi<TangleConfig, TangleClient> {
        let block_ref = BlockRef::from_hash(sp_core::hash::H256::from_slice(&at));
        self.client.runtime_api().at(block_ref)
    }
}

#[async_trait::async_trait]
impl Client<TangleEvent> for TangleRuntime {
    async fn next_event(&self) -> Option<FinalityNotification> {
        let mut lock = self
            .finality_notification_stream
            .try_lock_timeout(Duration::from_millis(500))
            .await
            .ok()?;
        match lock.as_mut() {
            Some(stream) => {
                let block = stream.next().await?.ok()?;
                let notification = FinalityNotification {
                    number: block.number().into(),
                    hash: block.hash().into(),
                };
                let mut lock2 = self
                    .latest_finality_notification
                    .lock_timeout(Duration::from_millis(500))
                    .await;
                *lock2 = Some(notification.clone());
                Some(notification)
            }
            None => {
                drop(lock);
                self.initialize().await.ok()?;
                // Next time, the stream should be initialized.
                self.next_event().await
            }
        }
    }

    async fn latest_event(&self) -> Option<FinalityNotification> {
        let lock = self
            .latest_finality_notification
            .try_lock_timeout(Duration::from_millis(500))
            .await
            .ok()?;
        match &*lock {
            Some(notification) => Some(notification.clone()),
            None => {
                drop(lock);
                self.next_event().await
            }
        }
    }
}

#[async_trait::async_trait]
impl ClientWithServicesApi<TangleEnvironment> for TangleRuntime {
    async fn query_jobs_by_validator(
        &self,
        at: [u8; 32],
        validator: AccountId32,
    ) -> core::result::Result<
        Option<
            Vec<
                RpcResponseJobsData<
                    AccountId32,
                    u64,
                    MaxParticipants,
                    MaxSubmissionLen,
                    MaxAdditionalParamsLen,
                >,
            >,
        >,
        gadget_common::Error,
    > {
        let q = api::apis()
            .services_api()
            .query_services_with_blueprints_by_operator(validator);
        //let q = api::apis().jobs_api().query_jobs_by_validator(validator);
        self.runtime_api(at)
            .call(q)
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })
            .await
    }

    async fn query_job_by_id(
        &self,
        at: [u8; 32],
        role_type: RoleType,
        job_id: u64,
    ) -> core::result::Result<
        Option<
            RpcResponseJobsData<
                AccountId32,
                u64,
                MaxParticipants,
                MaxSubmissionLen,
                MaxAdditionalParamsLen,
            >,
        >,
        gadget_common::Error,
    > {
        let q = api::apis().jobs_api().query_job_by_id(role_type, job_id);
        self.runtime_api(at)
            .call(q)
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })
            .await
    }

    async fn query_job_result(
        &self,
        at: [u8; 32],
        role_type: RoleType,
        job_id: u64,
    ) -> core::result::Result<
        Option<
            PhaseResult<
                AccountId32,
                u64,
                MaxParticipants,
                MaxKeyLen,
                MaxDataLen,
                MaxSignatureLen,
                MaxSubmissionLen,
                MaxProofLen,
                MaxAdditionalParamsLen,
            >,
        >,
        gadget_common::Error,
    > {
        let q = api::apis().jobs_api().query_job_result(role_type, job_id);
        self.runtime_api(at)
            .call(q)
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })
            .await
    }

    async fn query_next_job_id(
        &self,
        at: [u8; 32],
    ) -> core::result::Result<u64, gadget_common::Error> {
        let q = api::apis().jobs_api().query_next_job_id();
        self.runtime_api(at)
            .call(q)
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })
            .await
    }

    async fn query_restaker_role_key(
        &self,
        at: [u8; 32],
        address: AccountId32,
    ) -> core::result::Result<Option<Vec<u8>>, gadget_common::Error> {
        let q = api::apis().jobs_api().query_restaker_role_key(address);
        self.runtime_api(at)
            .call(q)
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })
            .await
    }

    async fn query_restaker_roles(
        &self,
        at: [u8; 32],
        address: AccountId32,
    ) -> Result<Vec<Blueprints>, gadget_common::Error> {
        let block_ref = BlockRef::from_hash(sp_core::hash::H256::from_slice(&at));
        let call = api::storage().services().user_services(address);

        let blueprint_ids: Vec<u64> = self
            .client
            .storage()
            .at(block_ref.clone())
            .fetch_or_default(&call)
            .map_ok(|v| v.0)
            .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })
            .await?;

        let mut ret = vec![];

        for blueprint_id in blueprint_ids {
            let svcs = api::storage().services().blueprints(blueprint_id);
            let blueprint: Blueprints = self
                .client
                .storage()
                .at(block_ref.clone())
                .fetch_or_default(&svcs)
                .map_err(|e| gadget_common::Error::ClientError { err: e.to_string() })
                .await?;

            ret.push(blueprint);
        }

        Ok(ret)
    }
}

#[cfg(test)]
#[cfg(not(target_family = "wasm"))]
mod tests {
    use super::*;
    use gadget_common::color_eyre::eyre::OptionExt;
    use gadget_common::gadget_io::tokio;

    #[ignore = "requires a running node"]
    #[tokio::test]
    async fn client() -> gadget_common::color_eyre::Result<()> {
        let subxt_client = subxt::OnlineClient::new().await?;
        let runtime = TangleRuntime::new(subxt_client);

        let notification = runtime
            .next_event()
            .await
            .ok_or_eyre("Finality notification not found")?;
        let job_id = runtime.query_next_job_id(notification.hash).await?;
        eprintln!("Next job id: {}", job_id);
        Ok(())
    }
}
