use std::sync::Arc;
use std::time::Duration;

use crate::clients::Client;
use crate::error::Error;
use crate::mutex_ext::TokioMutexExt;
use subxt::blocks::{Block, BlockRef};
use subxt::events::Events;
use subxt::utils::AccountId32;
use subxt::{self, SubstrateConfig};

/// The [Config](subxt::Config) providing the runtime types.
pub type TangleConfig = SubstrateConfig;
/// The client used to perform API calls, using the [TangleConfig].
pub type TangleClient = subxt::OnlineClient<TangleConfig>;
type TangleBlock = Block<TangleConfig, TangleClient>;
type TangleBlockStream = subxt::backend::StreamOfResults<TangleBlock>;

#[derive(Clone, Debug)]
pub struct TangleEvent {
    /// Finalized block number.
    pub number: u64,
    /// Finalized block header hash.
    pub hash: [u8; 32],
    /// Events
    pub events: Events<TangleConfig>,
}

#[derive(Clone, Debug)]
pub struct TangleRuntimeClient {
    client: TangleClient,
    finality_notification_stream: Arc<tokio::sync::Mutex<Option<TangleBlockStream>>>,
    latest_finality_notification: Arc<tokio::sync::Mutex<Option<TangleEvent>>>,
    account_id: AccountId32,
}

impl TangleRuntimeClient {
    /// Create a new Tangle runtime client from an RPC url.
    ///
    /// # Errors
    ///
    /// * `url` is not a valid URL.
    /// * `url` is not a secure (https:// or wss://) URL.
    /// * `url` cannot be resolved.
    pub async fn from_url<U: AsRef<str>>(url: U, account_id: AccountId32) -> Result<Self, Error> {
        let client = TangleClient::from_url(url).await?;
        Ok(Self::new(client, account_id))
    }

    /// Create a new Tangle runtime client from an existing [`TangleClient`].
    pub fn new(client: TangleClient, account_id: AccountId32) -> Self {
        Self {
            client,
            finality_notification_stream: Arc::new(tokio::sync::Mutex::new(None)),
            latest_finality_notification: Arc::new(tokio::sync::Mutex::new(None)),
            account_id,
        }
    }

    /// Get the associated [`TangleClient`]
    pub fn client(&self) -> TangleClient {
        self.client.clone()
    }

    pub fn runtime_api(
        &self,
        at: [u8; 32],
    ) -> subxt::runtime_api::RuntimeApi<TangleConfig, TangleClient> {
        let block_ref = BlockRef::from_hash(sp_core::hash::H256::from_slice(&at));
        self.client.runtime_api().at(block_ref)
    }

    /// Get the associated account ID
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gadget_sdk::clients::tangle::runtime::TangleRuntimeClient;
    /// use subxt::utils::AccountId32;
    ///
    /// # async fn foo() -> Result<(), gadget_sdk::Error>{
    /// let account_id = AccountId32::from([0; 32]);
    /// let client = TangleRuntimeClient::from_url("https://foo.bar", account_id).await?;
    ///
    /// assert_eq!(client.account_id(), &account_id);
    /// # Ok(()) }
    /// ```
    pub fn account_id(&self) -> &AccountId32 {
        &self.account_id
    }

    // Initialize the `TangleRuntimeClient` to listen for finality notifications.
    //
    // NOTE: This method must be called before using the instance.
    async fn initialize(&self) -> Result<(), Error> {
        let finality_notification_stream = self.client.blocks().subscribe_finalized().await?;
        *self.finality_notification_stream.lock().await = Some(finality_notification_stream);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Client<TangleEvent> for TangleRuntimeClient {
    async fn next_event(&self) -> Option<TangleEvent> {
        let mut lock = self
            .finality_notification_stream
            .try_lock_timeout(Duration::from_millis(500))
            .await
            .ok()?;
        match lock.as_mut() {
            Some(stream) => {
                let block = stream.next().await?.ok()?;
                let events = block.events().await.ok()?;
                let notification = TangleEvent {
                    number: block.number().into(),
                    hash: block.hash().into(),
                    events,
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

    async fn latest_event(&self) -> Option<TangleEvent> {
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
