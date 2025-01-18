use async_trait::async_trait;
use gadget_std as std;
use gadget_std::boxed::Box;
use gadget_std::collections::HashMap;
use gadget_std::fmt::Display;
use gadget_std::hash::Hash;
use gadget_std::ops::Add;
use gadget_std::sync::atomic::AtomicBool;
use gadget_std::sync::Arc;
use gadget_std::{eprintln, string::String, vec::Vec};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

const OUTBOUND_POLL: Duration = Duration::from_millis(100);
const INBOUND_POLL: Duration = Duration::from_millis(100);

#[async_trait]
pub trait MessageMetadata {
    type JobId: Display + Hash + Eq + Copy + Send + Sync + 'static;
    type PeerId: Display + Hash + Eq + Copy + Send + Sync + 'static;
    type MessageId: Add<usize, Output = Self::MessageId>
        + Eq
        + PartialEq
        + Display
        + Hash
        + Ord
        + PartialOrd
        + Copy
        + Send
        + Sync
        + 'static;

    fn job_id(&self) -> Self::JobId;
    fn source_id(&self) -> Self::PeerId;
    fn destination_id(&self) -> Self::PeerId;
    fn message_id(&self) -> Self::MessageId;
    fn contents(&self) -> &[u8];
}

#[async_trait]
pub trait NetworkMessagingIO {
    type Message: MessageMetadata + Send + Sync + 'static;

    async fn next_message(&self) -> Option<Payload<Self::Message>>;
    async fn send_message(&self, message: &Payload<Self::Message>) -> Result<(), NetworkError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Payload<M: MessageMetadata> {
    Ack {
        job_id: M::JobId,
        from_id: M::PeerId,
        message_id: M::MessageId,
    },
    Message(M),
}

#[derive(Debug)]
pub enum NetworkError {
    SendFailed(String),
    ConnectionError(String),
}

#[derive(Debug)]
pub enum BackendError {
    StorageError(String),
    NotFound,
    Stopped,
}

#[derive(Debug, Copy, Clone)]
pub enum DeliveryError {
    NoReceiver,
    ChannelClosed,
}

// Modified Backend trait to handle both outbound and inbound messages
#[async_trait]
pub trait Backend<M: MessageMetadata> {
    async fn store_outbound(&self, message: M) -> Result<(), BackendError>;
    async fn store_inbound(&self, message: M) -> Result<(), BackendError>;
    async fn clear_message(
        &self,
        peer_id: M::PeerId,
        job_id: M::JobId,
        message_id: M::MessageId,
    ) -> Result<(), BackendError>;
    async fn get_pending_outbound(&self) -> Result<Vec<M>, BackendError>;
    async fn get_pending_inbound(&self) -> Result<Vec<M>, BackendError>;
}

#[async_trait]
pub trait LocalDelivery<M: MessageMetadata> {
    async fn deliver(&self, message: M) -> Result<(), DeliveryError>;
}

// Add this new struct to track last ACKed message IDs
pub struct MessageTracker<M: MessageMetadata> {
    last_acked: HashMap<(M::JobId, M::PeerId), M::MessageId>,
}

impl<M: MessageMetadata> MessageTracker<M> {
    fn new() -> Self {
        Self {
            last_acked: HashMap::new(),
        }
    }

    fn update_ack(&mut self, job_id: M::JobId, peer_id: M::PeerId, msg_id: M::MessageId) {
        let key = (job_id, peer_id);
        match self.last_acked.get(&key) {
            Some(last_id) => {
                if msg_id == *last_id + 1usize {
                    let _ = self.last_acked.insert(key, msg_id);
                }
            }
            None => {
                // For the first message in a sequence, only accept if it's the initial message
                let _ = self.last_acked.insert(key, msg_id);
            }
        }
    }

    fn can_send(&self, job_id: &M::JobId, peer_id: &M::PeerId, msg_id: &M::MessageId) -> bool {
        match self.last_acked.get(&(*job_id, *peer_id)) {
            Some(last_id) => *msg_id == *last_id + 1usize,
            None => true, // If there is no message for this job/peer_id combo, send it
        }
    }
}

pub struct MessageSystem<M, B, L, N>
where
    M: MessageMetadata + Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
    B: Backend<M> + Send + Sync + 'static,
    L: LocalDelivery<M> + Send + Sync + 'static,
    N: NetworkMessagingIO<Message = M> + Send + Sync + 'static,
{
    backend: Arc<B>,
    local_delivery: Arc<L>,
    network: Arc<N>,
    is_running: Arc<AtomicBool>,
    tracker: Arc<RwLock<MessageTracker<M>>>,
}

impl<M, B, L, N> Clone for MessageSystem<M, B, L, N>
where
    M: MessageMetadata + Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
    B: Backend<M> + Send + Sync + 'static,
    L: LocalDelivery<M> + Send + Sync + 'static,
    N: NetworkMessagingIO<Message = M> + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            local_delivery: self.local_delivery.clone(),
            network: self.network.clone(),
            is_running: self.is_running.clone(),
            tracker: self.tracker.clone(),
        }
    }
}

impl<M, B, L, N> MessageSystem<M, B, L, N>
where
    M: MessageMetadata + Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
    B: Backend<M> + Send + Sync + 'static,
    L: LocalDelivery<M> + Send + Sync + 'static,
    N: NetworkMessagingIO<Message = M> + Send + Sync + 'static,
{
    pub fn new(backend: B, local_delivery: L, network: N) -> Self {
        let this = Self {
            backend: Arc::new(backend),
            local_delivery: Arc::new(local_delivery),
            network: Arc::new(network),
            is_running: Arc::new(AtomicBool::new(true)),
            tracker: Arc::new(RwLock::new(MessageTracker::new())),
        };

        this.spawn_background_tasks();

        this
    }

    fn spawn_background_tasks(&self) {
        // Spawn outbound processing task
        let self_clone = self.clone();
        let is_alive = self.is_running.clone();

        let outbound_handle = tokio::spawn(async move {
            loop {
                self_clone.process_outbound().await;
                sleep(OUTBOUND_POLL).await;
            }
        });

        // Spawn inbound processing task
        let self_clone = self.clone();
        let inbound_handle = tokio::spawn(async move {
            loop {
                self_clone.process_inbound().await;
                sleep(INBOUND_POLL).await;
            }
        });

        // Spawn network listener task
        let self_clone = self.clone();
        let network_io_handle = tokio::spawn(async move {
            self_clone.process_network_messages().await;
        });

        // Spawn a task that selects all three handles, and on any of them finishing, it will
        // set the atomic bool to false
        drop(tokio::spawn(async move {
            tokio::select! {
                _ = outbound_handle => {
                    gadget_logging::error!("Outbound processing task prematurely ended");
                },
                _ = inbound_handle => {
                    gadget_logging::error!("Inbound processing task prematurely ended");
                },
                _ = network_io_handle => {
                    gadget_logging::error!("Network IO task prematurely ended");
                },
            }

            is_alive.store(false, gadget_std::sync::atomic::Ordering::Relaxed);
        }));
    }

    async fn process_outbound(&self) {
        let pending_messages = match self.backend.get_pending_outbound().await {
            Ok(messages) => messages,
            Err(e) => {
                eprintln!("Failed to get pending outbound messages: {:?}", e);
                return;
            }
        };

        // Group messages by (JobId, PeerId) pair
        let mut grouped_messages: HashMap<(M::JobId, M::PeerId), Vec<M>> = HashMap::new();
        for msg in pending_messages {
            grouped_messages
                .entry((msg.job_id(), msg.destination_id()))
                .or_default()
                .push(msg);
        }

        // Process each group independently
        let tracker = self.tracker.read().await;
        for ((job_id, peer_id), mut messages) in grouped_messages {
            // Sort messages by MessageId
            messages.sort_by_key(MessageMetadata::message_id);

            // Find the first message we can send based on ACKs
            if let Some(msg) = messages
                .into_iter()
                .find(|m| tracker.can_send(&job_id, &peer_id, &m.message_id()))
            {
                if let Err(e) = self.network.send_message(&Payload::Message(msg)).await {
                    eprintln!("Failed to send message: {:?}", e);
                }
            }
        }
    }

    async fn process_inbound(&self) {
        let pending_messages = match self.backend.get_pending_inbound().await {
            Ok(messages) => messages,
            Err(e) => {
                eprintln!("Failed to get pending inbound messages: {:?}", e);
                return;
            }
        };

        // Sort the pending messages in order by MessageID
        let pending_messages: Vec<M> = pending_messages
            .into_iter()
            .sorted_by_key(MessageMetadata::message_id)
            .collect();

        for message in pending_messages {
            match self.local_delivery.deliver(message.clone()).await {
                Ok(()) => {
                    // Create and send ACK
                    if let Err(e) = self
                        .network
                        .send_message(&Self::create_ack_message(&message))
                        .await
                    {
                        gadget_logging::error!("Failed to send ACK: {e:?}");
                        continue;
                    }

                    // Clear delivered message from backend
                    if let Err(e) = self
                        .backend
                        .clear_message(message.source_id(), message.job_id(), message.message_id())
                        .await
                    {
                        gadget_logging::error!("Failed to clear delivered message: {e:?}");
                    }
                }
                Err(e) => {
                    gadget_logging::error!("Failed to deliver message: {e:?}");
                }
            }
        }
    }

    // Modify process_network_messages to update the tracker
    async fn process_network_messages(&self) {
        loop {
            if let Some(message) = self.network.next_message().await {
                match message {
                    Payload::Ack {
                        job_id,
                        from_id,
                        message_id,
                    } => {
                        // Update the tracker with the new ACK
                        let mut tracker = self.tracker.write().await;
                        tracker.update_ack(job_id, from_id, message_id);

                        if let Err(e) = self
                            .backend
                            .clear_message(from_id, job_id, message_id)
                            .await
                        {
                            gadget_logging::error!("Failed to clear ACKed message: {e:?}");
                        }
                    }
                    Payload::Message(msg) => {
                        if let Err(e) = self.backend.store_inbound(msg).await {
                            gadget_logging::error!("Failed to store inbound message: {e:?}");
                        }
                    }
                }
            }
        }
    }

    /// Send a message through the message system
    ///
    /// # Errors
    ///
    /// Returns `BackendError::Stopped` if the message system is not running.
    /// May also return other `BackendError` variants if the backend storage operation fails.
    pub async fn send_message(&self, message: M) -> Result<(), BackendError> {
        if self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
            self.backend.store_outbound(message).await
        } else {
            Err(BackendError::Stopped)
        }
    }

    fn create_ack_message(original_message: &M) -> Payload<M> {
        Payload::Ack {
            job_id: original_message.job_id(),
            from_id: original_message.source_id(),
            message_id: original_message.message_id(),
        }
    }
}

// Example InMemoryBackend implementation
pub struct InMemoryBackend<M: MessageMetadata> {
    outbound: Mailbox<M::JobId, M::PeerId, M::MessageId, M>,
    inbound: Mailbox<M::JobId, M::PeerId, M::MessageId, M>,
}

type Mailbox<JobId, PeerId, MessageId, Message> =
    Arc<RwLock<HashMap<(JobId, PeerId, MessageId), Message>>>;

impl<M: MessageMetadata> InMemoryBackend<M> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            outbound: Arc::new(RwLock::new(HashMap::new())),
            inbound: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<M: MessageMetadata> Default for InMemoryBackend<M> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<M: MessageMetadata + Clone + Send + Sync + 'static> Backend<M> for InMemoryBackend<M> {
    async fn store_outbound(&self, message: M) -> Result<(), BackendError> {
        let mut outbound = self.outbound.write().await;
        let (job_id, source_id, message_id) =
            (message.job_id(), message.source_id(), message.message_id());

        if outbound
            .insert(
                (
                    message.job_id(),
                    message.destination_id(),
                    message.message_id(),
                ),
                message,
            )
            .is_some()
        {
            gadget_logging::warn!(
                "Overwriting existing message in outbound storage jid={}/dest={}/id={}",
                job_id,
                source_id,
                message_id
            );
        }
        Ok(())
    }

    async fn store_inbound(&self, message: M) -> Result<(), BackendError> {
        let mut inbound = self.inbound.write().await;
        let (job_id, source_id, message_id) =
            (message.job_id(), message.source_id(), message.message_id());

        if inbound
            .insert(
                (message.job_id(), message.source_id(), message.message_id()),
                message,
            )
            .is_some()
        {
            gadget_logging::warn!(
                "Overwriting existing message in inbound storage jid={}/src={}/id={}",
                job_id,
                source_id,
                message_id
            );
        }
        Ok(())
    }

    async fn clear_message(
        &self,
        peer_id: M::PeerId,
        job_id: M::JobId,
        message_id: M::MessageId,
    ) -> Result<(), BackendError> {
        // Try to remove from both outbound and inbound
        let mut outbound = self.outbound.write().await;
        let mut inbound = self.inbound.write().await;

        let _ = outbound.remove(&(job_id, peer_id, message_id));
        let _ = inbound.remove(&(job_id, peer_id, message_id));

        Ok(())
    }

    async fn get_pending_outbound(&self) -> Result<Vec<M>, BackendError> {
        let outbound = self.outbound.read().await;
        Ok(outbound.values().cloned().collect())
    }

    async fn get_pending_inbound(&self) -> Result<Vec<M>, BackendError> {
        let inbound = self.inbound.read().await;
        Ok(inbound.values().cloned().collect())
    }
}
