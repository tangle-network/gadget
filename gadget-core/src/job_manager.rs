use crate::job::ExecutableJob;
use parking_lot::RwLock;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::ops::{Add, Sub};
use std::{
    collections::{HashMap, VecDeque},
    hash::{Hash, Hasher},
    sync::Arc,
};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PollMethod {
    Interval { millis: u64 },
    Manual,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DeliveryType {
    EnqueuedProtocol,
    ActiveProtocol,
    // Protocol for the message is not yet available
    EnqueuedMessage,
}
pub struct ProtocolWorkManager<WM: WorkManagerInterface> {
    inner: Arc<RwLock<WorkManagerInner<WM>>>,
    pub utility: Arc<WM>,
    // for now, use a hard-coded value for the number of tasks
    max_tasks: Arc<usize>,
    max_enqueued_tasks: Arc<usize>,
    poll_method: Arc<PollMethod>,
}

impl<WM: WorkManagerInterface> Clone for ProtocolWorkManager<WM> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            utility: self.utility.clone(),
            max_tasks: self.max_tasks.clone(),
            max_enqueued_tasks: self.max_enqueued_tasks.clone(),
            poll_method: self.poll_method.clone(),
        }
    }
}

pub struct WorkManagerInner<WM: WorkManagerInterface> {
    pub active_tasks: HashMap<WM::TaskID, Job<WM>>,
    pub enqueued_tasks: VecDeque<Job<WM>>,
    // task hash => SSID => enqueued messages
    pub enqueued_messages: EnqueuedMessage<WM::TaskID, WM::RetryID, WM::ProtocolMessage>,
    pub retry_id_tracker: HashMap<WM::TaskID, WM::RetryID>,
}

pub type EnqueuedMessage<A, B, C> = HashMap<A, HashMap<B, VecDeque<C>>>;

pub trait WorkManagerInterface: Send + Sync + 'static + Sized {
    type RetryID: Copy + Hash + Eq + PartialEq + Send + Sync + 'static;
    type UserID: Copy + Hash + Eq + PartialEq + Send + Sync + 'static;
    type Clock: Copy
        + Debug
        + Default
        + Eq
        + Ord
        + PartialOrd
        + PartialEq
        + Send
        + Sync
        + Sub<Output = Self::Clock>
        + Add<Output = Self::Clock>
        + 'static;
    type ProtocolMessage: ProtocolMessageMetadata<Self> + Send + Sync + Clone + 'static;
    type Error: Debug + Send + Sync + 'static;
    type SessionID: Copy + Hash + Eq + PartialEq + Display + Debug + Send + Sync + 'static;
    type TaskID: Copy + Hash + Eq + PartialEq + Debug + Send + Sync + AsRef<[u8]> + 'static;

    fn debug(&self, input: String);
    fn error(&self, input: String);
    fn warn(&self, input: String);
    fn clock(&self) -> Self::Clock;
    fn acceptable_block_tolerance() -> Self::Clock;
    fn associated_block_id_acceptable(expected: Self::Clock, received: Self::Clock) -> bool {
        // Favor explicit logic for readability
        let tolerance = Self::acceptable_block_tolerance();
        let is_acceptable_above = received >= expected && received <= expected + tolerance;
        let is_acceptable_below =
            received < expected && received >= saturating_sub(expected, tolerance);
        let is_equal = expected == received;

        is_acceptable_above || is_acceptable_below || is_equal
    }
}

fn saturating_sub<T: Sub<Output = T> + Ord + Default>(a: T, b: T) -> T {
    if a < b {
        T::default()
    } else {
        a - b
    }
}

pub trait ProtocolMessageMetadata<WM: WorkManagerInterface> {
    fn associated_block_id(&self) -> WM::Clock;
    fn associated_session_id(&self) -> WM::SessionID;
    fn associated_retry_id(&self) -> WM::RetryID;
    fn associated_task(&self) -> WM::TaskID;
    fn associated_sender_user_id(&self) -> WM::UserID;
    fn associated_recipient_user_id(&self) -> Option<WM::UserID>;
}

/// The [`ProtocolRemote`] is the interface between the [`ProtocolWorkManager`] and the async protocol.
/// It *must* be unique between each async protocol.
pub trait ProtocolRemote<WM: WorkManagerInterface>: Send + Sync + 'static {
    fn start(&self) -> Result<(), WM::Error>;
    fn session_id(&self) -> WM::SessionID;
    fn started_at(&self) -> WM::Clock;
    fn shutdown(&self, reason: ShutdownReason) -> Result<(), WM::Error>;
    fn is_done(&self) -> bool;
    fn deliver_message(&self, message: WM::ProtocolMessage) -> Result<(), WM::Error>;
    fn has_started(&self) -> bool;
    fn retry_id(&self) -> WM::RetryID;

    fn has_stalled(&self, now: WM::Clock) -> bool {
        now >= self.started_at() + WM::acceptable_block_tolerance()
    }

    fn is_active(&self) -> bool {
        // If the protocol has started, is not done, and has not stalled, then it is active
        self.has_started() && !self.is_done() && !self.has_started()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct JobMetadata<WM: WorkManagerInterface> {
    pub session_id: WM::SessionID,
    pub is_stalled: bool,
    pub is_finished: bool,
    pub has_started: bool,
    pub is_active: bool,
}

impl<WM: WorkManagerInterface> ProtocolWorkManager<WM> {
    pub fn new(
        utility: WM,
        max_tasks: usize,
        max_enqueued_tasks: usize,
        poll_method: PollMethod,
    ) -> Self {
        let this = Self {
            inner: Arc::new(RwLock::new(WorkManagerInner {
                active_tasks: HashMap::new(),
                enqueued_tasks: VecDeque::new(),
                enqueued_messages: HashMap::new(),
                retry_id_tracker: HashMap::new(),
            })),
            utility: Arc::new(utility),
            max_tasks: Arc::new(max_tasks),
            max_enqueued_tasks: Arc::new(max_enqueued_tasks),
            poll_method: Arc::new(poll_method),
        };

        if let PollMethod::Interval { millis } = poll_method {
            let this_worker = this.clone();
            let logger = this_worker.utility.clone();

            let handler = async move {
                let periodic_poller = async move {
                    let mut interval =
                        tokio::time::interval(std::time::Duration::from_millis(millis));
                    loop {
                        interval.tick().await;
                        this_worker.poll();
                    }
                };

                periodic_poller.await;
                logger.error("[worker] periodic_poller exited".to_string());
            };

            tokio::task::spawn(handler);
        }

        this
    }

    pub fn clear_enqueued_tasks(&self) {
        let mut lock = self.inner.write();
        lock.enqueued_tasks.clear();
    }

    /// Pushes the task, but does not necessarily start it
    pub fn push_task<T: ExecutableJob>(
        &self,
        task_hash: WM::TaskID,
        force_start: bool,
        handle: Arc<dyn ProtocolRemote<WM>>,
        task: T,
    ) -> Result<(), WorkManagerError> {
        if self.job_exists(&task_hash) {
            return Err(WorkManagerError::PushTaskFailed {
                reason: "Task already exists".to_string(),
            });
        }

        let mut lock = self.inner.write();
        let job = Job {
            task: Arc::new(parking_lot::Mutex::new(Some(Box::new(task)))),
            handle,
            task_hash,
            utility: self.utility.clone(),
            recorded_messages: vec![],
        };

        if force_start {
            // This job has priority over the max_tasks limit
            self.utility.debug(format!(
                "[FORCE START] Force starting task {}",
                hex::encode(task_hash)
            ));
            return self.start_job_unconditional(job, &mut *lock);
        }

        if lock.enqueued_tasks.len() + lock.active_tasks.len()
            >= *self.max_enqueued_tasks + *self.max_tasks
        {
            return Err(WorkManagerError::PushTaskFailed {
                reason: "Too many active and enqueued tasks".to_string(),
            });
        }

        lock.enqueued_tasks.push_back(job);

        drop(lock);

        if *self.poll_method != PollMethod::Manual {
            self.poll();
        }

        Ok(())
    }

    pub fn can_submit_more_tasks(&self) -> bool {
        let lock = self.inner.read();
        lock.enqueued_tasks.len() + lock.active_tasks.len()
            < *self.max_enqueued_tasks + *self.max_tasks
    }

    // Only relevant for keygen
    pub fn get_active_sessions_metadata(&self, now: WM::Clock) -> Vec<JobMetadata<WM>> {
        self.inner
            .read()
            .active_tasks
            .iter()
            .map(|r| r.1.metadata(now))
            .collect()
    }

    // This will shutdown and drop all tasks and enqueued messages
    pub fn force_shutdown_all(&self) {
        let mut lock = self.inner.write();
        lock.active_tasks.clear();
        lock.enqueued_tasks.clear();
        lock.enqueued_messages.clear();
    }

    pub fn poll(&self) {
        // Go through each task and see if it's done
        let now = self.utility.clock();
        let mut lock = self.inner.write();
        let cur_count = lock.active_tasks.len();
        lock.active_tasks.retain(|_, job| {
            let is_stalled = job.handle.has_stalled(now);
            let is_done = job.handle.is_done();

            if is_stalled && !is_done {
                // If stalled, lets log the start and now blocks for logging purposes
                self.utility.debug(format!(
                    "[worker] Job {:?} | Started at {:?} | Now {:?} | is stalled, shutting down",
                    hex::encode(job.task_hash),
                    job.handle.started_at(),
                    now
                ));

                // The task is stalled, lets be pedantic and shutdown
                let _ = job.handle.shutdown(ShutdownReason::Stalled);
                // Return false so that the proposals are released from the currently signing
                // proposals
                return false;
            }

            let is_done = job.handle.is_done();

            !is_done
        });

        let new_count = lock.active_tasks.len();
        if cur_count != new_count {
            self.utility
                .debug(format!("[worker] {} jobs dropped", cur_count - new_count));
        }

        // Now, check to see if there is room to start a new task
        let tasks_to_start = self.max_tasks.saturating_sub(lock.active_tasks.len());
        for _ in 0..tasks_to_start {
            if let Some(job) = lock.enqueued_tasks.pop_front() {
                let task_hash = job.task_hash;
                if let Err(err) = self.start_job_unconditional(job, &mut *lock) {
                    self.utility.error(format!(
                        "[worker] Failed to start job {:?}: {err:?}",
                        hex::encode(task_hash)
                    ));
                }
            } else {
                break;
            }
        }

        // Next, remove any outdated enqueued messages to prevent RAM bloat
        let mut to_remove = vec![];
        for (hash, queue) in lock.enqueued_messages.iter_mut() {
            for (retry_id, queue) in queue.iter_mut() {
                let before = queue.len();
                // Only keep the messages that are not outdated
                queue.retain(|msg| {
                    WM::associated_block_id_acceptable(now, msg.associated_block_id())
                });
                let after = queue.len();

                if before != after {
                    self.utility.debug(format!(
                        "[worker] Removed {} outdated enqueued messages from the queue for {:?}",
                        before - after,
                        hex::encode(*hash)
                    ));
                }

                if queue.is_empty() {
                    to_remove.push((*hash, *retry_id));
                }
            }
        }

        // Next, to prevent the existence of piling-up empty *inner* queues, remove them
        for (hash, retry_id) in to_remove {
            lock.enqueued_messages
                .get_mut(&hash)
                .expect("Should be available")
                .remove(&retry_id);
        }

        // Finally, remove any empty outer maps
        lock.enqueued_messages.retain(|_, v| !v.is_empty());
    }

    fn start_job_unconditional(
        &self,
        job: Job<WM>,
        lock: &mut WorkManagerInner<WM>,
    ) -> Result<(), WorkManagerError> {
        self.utility.debug(format!(
            "[worker] Starting job {:?}",
            hex::encode(job.task_hash)
        ));
        if let Err(err) = job.handle.start() {
            return Err(WorkManagerError::PushTaskFailed {
                reason: format!(
                    "Failed to start job {:?}: {err:?}",
                    hex::encode(job.task_hash)
                ),
            });
        } else {
            // deliver all the enqueued messages to the protocol now
            if let Some(mut enqueued_messages_map) = lock.enqueued_messages.remove(&job.task_hash) {
                let job_retry_id = job.handle.retry_id();
                if let Some(mut enqueued_messages) = enqueued_messages_map.remove(&job_retry_id) {
                    self.utility.debug(format!(
                        "Will now deliver {} enqueued message(s) to the async protocol for {:?}",
                        enqueued_messages.len(),
                        hex::encode(job.task_hash)
                    ));

                    while let Some(message) = enqueued_messages.pop_front() {
                        if should_deliver(&job, &message, job.task_hash) {
                            if let Err(err) = job.handle.deliver_message(message.clone()) {
                                self.utility.error(format!(
                                    "Unable to deliver message for job {:?}: {err:?}",
                                    hex::encode(job.task_hash)
                                ));
                            } else {
                                lock.record_message(message)
                            }
                        } else {
                            self.utility.warn("Will not deliver enqueued message to async protocol since the message is no longer acceptable".to_string())
                        }
                    }
                }

                // If there are any other messages for other SSIDs, put them back in the map
                if !enqueued_messages_map.is_empty() {
                    lock.enqueued_messages
                        .insert(job.task_hash, enqueued_messages_map);
                }
            }
        }
        let task = job.task.clone();
        // Put the job inside here, that way the drop code does not get called right away,
        // killing the process
        let task_hash = job.task_hash;
        *lock
            .retry_id_tracker
            .entry(task_hash)
            .or_insert(job.handle.retry_id()) = job.handle.retry_id();
        lock.active_tasks.insert(task_hash, job);

        let mut task = task.lock().take().expect("Should not happen");
        let logger = self.utility.clone();
        let on_task_finish_handle = self.inner.clone();
        let task = async move {
            let result = task.execute().await;
            if let Err(err) = result {
                logger.error(format!(
                    "[worker] Job {:?} finished with error: {err:?}",
                    hex::encode(task_hash)
                ));
            } else {
                logger.debug(format!(
                    "[worker] Job {:?} finished successfully",
                    hex::encode(task_hash)
                ));

                on_task_finish_handle
                    .write()
                    .retry_id_tracker
                    .remove(&task_hash);
            }
        };

        // Spawn the task. When it finishes, it will clean itself up
        tokio::task::spawn(task);
        Ok(())
    }

    pub fn latest_retry_id(&self, task_hash: &WM::TaskID) -> Option<WM::RetryID> {
        let lock = self.inner.read();
        lock.retry_id_tracker.get(task_hash).copied()
    }

    pub fn job_exists(&self, job: &WM::TaskID) -> bool {
        let lock = self.inner.read();
        lock.active_tasks.iter().any(|r| &r.1.task_hash == job)
            || lock.enqueued_tasks.iter().any(|j| &j.task_hash == job)
    }

    pub fn deliver_message(
        &self,
        msg: WM::ProtocolMessage,
    ) -> Result<DeliveryType, WorkManagerError> {
        if let Some(recv) = msg.associated_recipient_user_id() {
            if recv == msg.associated_sender_user_id() {
                self.utility
                    .warn("Message sent to self, which is not allowed".to_string());
                return Err(WorkManagerError::DeliverMessageFailed {
                    reason: "Message sent to self, which is not allowed".to_string(),
                });
            }
        }

        let message_task_hash = msg.associated_task();
        let mut lock = self.inner.write();

        // Check the enqueued
        for task in lock.enqueued_tasks.iter() {
            if should_deliver(task, &msg, message_task_hash) {
                self.utility.debug(format!(
                    "Message is for this ENQUEUED execution in session: {}",
                    task.handle.session_id()
                ));
                if let Err(err) = task.handle.deliver_message(msg.clone()) {
                    return Err(WorkManagerError::DeliverMessageFailed {
                        reason: format!("{err:?}"),
                    });
                }

                lock.record_message(msg);
                return Ok(DeliveryType::EnqueuedProtocol);
            }
        }

        // Check the currently signing
        for (_, task) in lock.active_tasks.iter() {
            if should_deliver(task, &msg, message_task_hash) {
                self.utility.debug(format!(
                    "Message is for this CURRENT execution in session: {}",
                    task.handle.session_id()
                ));
                if let Err(err) = task.handle.deliver_message(msg.clone()) {
                    return Err(WorkManagerError::DeliverMessageFailed {
                        reason: format!("{err:?}"),
                    });
                }

                lock.record_message(msg);
                return Ok(DeliveryType::ActiveProtocol);
            }
        }

        // if the protocol is neither started nor enqueued, then, this message may be for a future
        // async protocol. Store the message
        let current_running_session_ids: Vec<WM::SessionID> = lock
            .active_tasks
            .iter()
            .map(|job| job.1.handle.session_id())
            .collect();
        let enqueued_session_ids: Vec<WM::SessionID> = lock
            .enqueued_tasks
            .iter()
            .map(|job| job.handle.session_id())
            .collect();
        self.utility
            .debug(format!("Enqueuing message for {:?} | current_running_session_ids: {current_running_session_ids:?} | enqueued_session_ids: {enqueued_session_ids:?}", hex::encode(message_task_hash)));
        lock.enqueued_messages
            .entry(message_task_hash)
            .or_default()
            .entry(msg.associated_retry_id())
            .or_default()
            .push_back(msg);

        Ok(DeliveryType::EnqueuedMessage)
    }

    #[cfg(test)]
    pub fn simulate_job_stall(&self, task_id: &WM::TaskID)
    where
        WM::RetryID: std::ops::AddAssign<u32>,
    {
        let mut lock = self.inner.write();
        lock.active_tasks.retain(|_, job| {
            let should_keep = &job.task_hash != task_id;

            if !should_keep {
                let _ = job.handle.shutdown(ShutdownReason::Stalled);
            }

            should_keep
        });

        *lock.retry_id_tracker.get_mut(task_id).expect("Must exist") += 1;
    }

    pub fn visit_recorded_messages<V: FnOnce(&Vec<WM::ProtocolMessage>) -> T, T>(
        &self,
        task_hash: &WM::TaskID,
        visitor: V,
    ) -> Option<T> {
        let lock = self.inner.read();
        if let Some(active_job) = lock.active_tasks.get(task_hash) {
            return Some(visitor(&active_job.recorded_messages));
        }

        if let Some(enqueued_job) = lock
            .enqueued_tasks
            .iter()
            .find(|j| j.task_hash == *task_hash)
        {
            return Some(visitor(&enqueued_job.recorded_messages));
        }

        None
    }

    pub fn poll_method(&self) -> PollMethod {
        *self.poll_method
    }
}

impl<WM: WorkManagerInterface> WorkManagerInner<WM> {
    fn record_message(&mut self, msg: WM::ProtocolMessage) {
        if let Some(active_job) = self.active_tasks.get_mut(&msg.associated_task()) {
            active_job.recorded_messages.push(msg);
            return;
        }

        if let Some(enqueued_job) = self
            .enqueued_tasks
            .iter_mut()
            .find(|j| j.task_hash == msg.associated_task())
        {
            enqueued_job.recorded_messages.push(msg);
        }
    }
}

pub struct Job<WM: WorkManagerInterface> {
    task_hash: WM::TaskID,
    utility: Arc<WM>,
    handle: Arc<dyn ProtocolRemote<WM>>,
    task: Arc<parking_lot::Mutex<Option<Box<dyn ExecutableJob>>>>,
    recorded_messages: Vec<WM::ProtocolMessage>,
}

impl<WM: WorkManagerInterface> Job<WM> {
    fn metadata(&self, now: WM::Clock) -> JobMetadata<WM> {
        JobMetadata::<WM> {
            session_id: self.handle.session_id(),
            is_stalled: self.handle.has_stalled(now),
            is_finished: self.handle.is_done(),
            has_started: self.handle.has_started(),
            is_active: self.handle.is_active(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum WorkManagerError {
    PushTaskFailed { reason: String },
    DeliverMessageFailed { reason: String },
}

#[derive(Debug)]
pub enum ShutdownReason {
    Stalled,
    DropCode,
}

pub trait SendFuture<'a, T>: Send + Future<Output = T> + 'a {}
impl<'a, F: Send + Future<Output = T> + 'a, T> SendFuture<'a, T> for F {}

impl<WM: WorkManagerInterface> PartialEq for Job<WM> {
    fn eq(&self, other: &Self) -> bool {
        self.task_hash == other.task_hash
    }
}

impl<WM: WorkManagerInterface> Eq for Job<WM> {}

impl<WM: WorkManagerInterface> Hash for Job<WM> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.task_hash.hash(state);
    }
}

impl<WM: WorkManagerInterface> Drop for Job<WM> {
    fn drop(&mut self) {
        self.utility.debug(format!(
            "Will remove job {:?} from JobManager",
            hex::encode(self.task_hash)
        ));
        let _ = self.handle.shutdown(ShutdownReason::DropCode);
    }
}

fn should_deliver<WM: WorkManagerInterface>(
    task: &Job<WM>,
    msg: &WM::ProtocolMessage,
    message_task_hash: WM::TaskID,
) -> bool {
    task.handle.session_id() == msg.associated_session_id()
        && task.task_hash == message_task_hash
        && task.handle.retry_id() == msg.associated_retry_id()
        && WM::associated_block_id_acceptable(
            task.handle.started_at(), // use to be associated_block_id
            msg.associated_block_id(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
    use parking_lot::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

    #[derive(Debug, Eq, PartialEq)]
    struct TestWorkManager;
    #[derive(Clone, Eq, PartialEq, Debug)]
    pub struct TestMessage {
        message: String,
        associated_block_id: u64,
        associated_session_id: u32,
        associated_retry_id: u32,
        associated_task: [u8; 32],
        associated_sender: u64,
        associated_recipient: Option<u64>,
    }

    impl ProtocolMessageMetadata<TestWorkManager> for TestMessage {
        fn associated_block_id(&self) -> u64 {
            self.associated_block_id
        }
        fn associated_session_id(&self) -> u32 {
            self.associated_session_id
        }
        fn associated_retry_id(&self) -> u32 {
            self.associated_retry_id
        }
        fn associated_task(&self) -> [u8; 32] {
            self.associated_task
        }

        fn associated_sender_user_id(&self) -> <TestWorkManager as WorkManagerInterface>::UserID {
            self.associated_sender
        }

        fn associated_recipient_user_id(
            &self,
        ) -> Option<<TestWorkManager as WorkManagerInterface>::UserID> {
            self.associated_recipient
        }
    }

    impl WorkManagerInterface for TestWorkManager {
        type RetryID = u32;
        type UserID = u64;
        type Clock = u64;
        type ProtocolMessage = TestMessage;
        type Error = ();
        type SessionID = u32;
        type TaskID = [u8; 32];

        fn debug(&self, input: String) {
            log::debug!("{input}")
        }
        fn error(&self, input: String) {
            log::error!("{input}")
        }
        fn warn(&self, input: String) {
            log::warn!("{input}")
        }
        fn clock(&self) -> Self::Clock {
            0
        }

        fn acceptable_block_tolerance() -> Self::Clock {
            0
        }
    }

    #[derive(Clone)]
    pub struct TestProtocolRemote {
        session_id: u32,
        retry_id: u32,
        started_at: u64,
        delivered_messages: UnboundedSender<TestMessage>,
        start_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
        is_done: Arc<AtomicBool>,
        has_started: Arc<AtomicBool>,
    }

    impl TestProtocolRemote {
        pub fn new(
            session_id: u32,
            retry_id: u32,
            started_at: u64,
            task_tx: UnboundedSender<TestMessage>,
            start_tx: tokio::sync::oneshot::Sender<()>,
            is_done: Arc<AtomicBool>,
        ) -> Arc<Self> {
            Arc::new(Self {
                session_id,
                retry_id,
                started_at,
                delivered_messages: task_tx,
                start_tx: Arc::new(Mutex::new(Some(start_tx))),
                is_done,
                has_started: Arc::new(AtomicBool::new(false)),
            })
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn generate_async_protocol(
        session_id: u32,
        retry_id: u32,
        started_at: u64,
    ) -> (
        Arc<TestProtocolRemote>,
        BuiltExecutableJobWrapper,
        UnboundedReceiver<TestMessage>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (start_tx, start_rx) = tokio::sync::oneshot::channel();
        let is_done = Arc::new(AtomicBool::new(false));
        let remote = TestProtocolRemote::new(
            session_id,
            retry_id,
            started_at,
            tx,
            start_tx,
            is_done.clone(),
        );
        let protocol = async move {
            start_rx.await.unwrap();
            is_done.store(true, Ordering::SeqCst);
            Ok(())
        };
        let task = JobBuilder::new().protocol(protocol).build();
        (remote, task, rx)
    }

    #[allow(clippy::type_complexity)]
    pub fn generate_async_protocol_error(
        session_id: u32,
        retry_id: u32,
        started_at: u64,
    ) -> (
        Arc<TestProtocolRemote>,
        BuiltExecutableJobWrapper,
        UnboundedReceiver<TestMessage>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (start_tx, start_rx) = tokio::sync::oneshot::channel();
        let is_done = Arc::new(AtomicBool::new(false));
        let remote = TestProtocolRemote::new(
            session_id,
            retry_id,
            started_at,
            tx,
            start_tx,
            is_done.clone(),
        );
        let protocol = async move {
            start_rx.await.unwrap();
            is_done.store(true, Ordering::SeqCst);
            Err(JobError {
                reason: "Intentional error".to_string(),
            })
        };
        let task = JobBuilder::new().protocol(protocol).build();
        (remote, task, rx)
    }

    impl ProtocolRemote<TestWorkManager> for TestProtocolRemote {
        fn start(&self) -> Result<(), ()> {
            self.start_tx.lock().take().unwrap().send(())?;
            self.has_started.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn session_id(&self) -> u32 {
            self.session_id
        }
        fn has_stalled(&self, now: u64) -> bool {
            now > self.started_at
        }
        fn started_at(&self) -> u64 {
            self.started_at
        }
        fn shutdown(&self, _reason: ShutdownReason) -> Result<(), ()> {
            Ok(())
        }
        fn is_done(&self) -> bool {
            self.is_done.load(Ordering::SeqCst)
        }
        fn deliver_message(&self, message: TestMessage) -> Result<(), ()> {
            self.delivered_messages.send(message).map_err(|_| ())
        }
        fn has_started(&self) -> bool {
            self.has_started.load(Ordering::SeqCst)
        }
        fn is_active(&self) -> bool {
            self.has_started() && !self.is_done()
        }
        fn retry_id(&self) -> u32 {
            self.retry_id
        }
    }

    #[tokio::test]
    async fn test_push_task() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote, task, _rx) = generate_async_protocol(1, 1, 0);
        let result = work_manager.push_task([0; 32], false, remote, task);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_deliver_message() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote, task, mut rx) = generate_async_protocol(0, 0, 0);

        work_manager
            .push_task(Default::default(), true, remote.clone(), task)
            .unwrap();

        let message = TestMessage {
            message: "test".to_string(),
            associated_block_id: 0,
            associated_session_id: 0,
            associated_retry_id: 0,
            associated_task: [0; 32],
            associated_recipient: None,
            associated_sender: 0,
        };
        assert_ne!(
            DeliveryType::EnqueuedMessage,
            work_manager.deliver_message(message).unwrap()
        );
        let _ = rx.recv().await.unwrap();
    }

    #[tokio::test]
    async fn test_deliver_self_ref_message() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote, task, _rx) = generate_async_protocol(0, 0, 0);

        work_manager
            .push_task(Default::default(), true, remote.clone(), task)
            .unwrap();

        let message = TestMessage {
            message: "test".to_string(),
            associated_block_id: 0,
            associated_session_id: 0,
            associated_retry_id: 0,
            associated_task: [0; 32],
            associated_recipient: Some(0),
            associated_sender: 0,
        };
        assert!(work_manager.deliver_message(message).is_err());
    }

    #[tokio::test]
    async fn test_job_exists() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote, task, _rx) = generate_async_protocol(1, 1, 0);

        let result = work_manager.job_exists(&[0; 32]);
        assert!(!result);

        work_manager.push_task([0; 32], true, remote, task).unwrap();

        let result = work_manager.job_exists(&[0; 32]);
        assert!(result);
    }

    #[tokio::test]
    async fn test_add_multiple_tasks() {
        let work_manager = ProtocolWorkManager::new(
            TestWorkManager,
            2, // max 2 tasks
            0,
            PollMethod::Manual,
        );

        let (remote1, task1, _rx) = generate_async_protocol(1, 1, 0);
        let (remote2, task2, _rx) = generate_async_protocol(2, 2, 0);
        let (remote3, task3, _rx) = generate_async_protocol(3, 3, 0);

        // Add 2 tasks, should succeed
        assert!(work_manager
            .push_task([1; 32], false, remote1, task1)
            .is_ok());
        assert!(work_manager
            .push_task([2; 32], false, remote2, task2)
            .is_ok());

        // Try to add a third, should fail
        assert!(work_manager
            .push_task([3; 32], false, remote3, task3)
            .is_err());
    }

    #[tokio::test]
    async fn test_deliver_to_queued_task() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote, task, mut rx) = generate_async_protocol(1, 1, 0);

        // Add a queued task
        work_manager
            .push_task([0; 32], false, remote.clone(), task)
            .unwrap();

        // Deliver message, should succeed
        let msg = TestMessage {
            message: "test".to_string(),
            associated_block_id: 0,
            associated_session_id: 1,
            associated_retry_id: 1,
            associated_task: [0; 32],
            associated_recipient: None,
            associated_sender: 0,
        };
        assert_ne!(
            DeliveryType::EnqueuedMessage,
            work_manager.deliver_message(msg.clone()).unwrap()
        );
        let next_message = rx.recv().await.unwrap();
        assert_eq!(next_message, msg);
    }

    #[tokio::test]
    async fn test_get_task_metadata() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote1, task1, _rx) = generate_async_protocol(1, 1, 0);
        let (remote2, task2, _rx) = generate_async_protocol(2, 2, 0);

        work_manager
            .push_task([1; 32], true, remote1, task1)
            .unwrap();
        work_manager
            .push_task([2; 32], true, remote2, task2)
            .unwrap();

        let now = 0;
        let metadata = work_manager.get_active_sessions_metadata(now);

        assert_eq!(metadata.len(), 2);
        let expected1 = JobMetadata {
            session_id: 1,
            is_stalled: false,
            is_finished: false,
            has_started: true,
            is_active: true,
        };

        let expected2 = JobMetadata {
            session_id: 2,
            is_stalled: false,
            is_finished: false,
            has_started: true,
            is_active: true,
        };

        assert!(metadata.contains(&expected1));
        assert!(metadata.contains(&expected2));

        // Now, start the tasks
        work_manager.poll();
        // Wait some time for the tasks to finish
        tokio::time::sleep(Duration::from_millis(100)).await;
        // Poll again to cleanup
        work_manager.poll();
        // Re-check the statuses
        let metadata = work_manager.get_active_sessions_metadata(now);

        assert!(metadata.is_empty());
    }

    #[tokio::test]
    async fn test_get_task_metadata_no_force_start() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote1, task1, _rx) = generate_async_protocol(1, 1, 0);
        let (remote2, task2, _rx) = generate_async_protocol(2, 2, 0);

        let now = 0;

        work_manager
            .push_task([1; 32], false, remote1, task1)
            .unwrap();
        work_manager
            .push_task([2; 32], false, remote2, task2)
            .unwrap();

        let metadata = work_manager.get_active_sessions_metadata(now);
        assert!(metadata.is_empty());

        // Now, poll to start the tasks
        work_manager.poll();

        let metadata = work_manager.get_active_sessions_metadata(now);

        assert_eq!(metadata.len(), 2);
        let expected1 = JobMetadata {
            session_id: 1,
            is_stalled: false,
            is_finished: false,
            has_started: true,
            is_active: true,
        };

        let expected2 = JobMetadata {
            session_id: 2,
            is_stalled: false,
            is_finished: false,
            has_started: true,
            is_active: true,
        };

        assert!(metadata.contains(&expected1));
        assert!(metadata.contains(&expected2));

        // Wait some time for the tasks to finish
        tokio::time::sleep(Duration::from_millis(100)).await;
        // Poll again to cleanup
        work_manager.poll();
        // Re-check the statuses
        let metadata = work_manager.get_active_sessions_metadata(now);

        assert!(metadata.is_empty());
    }

    #[tokio::test]
    async fn test_force_shutdown_all() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote1, task1, _rx) = generate_async_protocol(1, 1, 0);
        let (remote2, task2, _rx) = generate_async_protocol(2, 2, 0);
        work_manager
            .push_task([1; 32], true, remote1, task1)
            .unwrap();
        work_manager
            .push_task([2; 32], true, remote2, task2)
            .unwrap();

        // Verify that the tasks were added
        assert!(work_manager.job_exists(&[1; 32]));
        assert!(work_manager.job_exists(&[2; 32]));

        // Force shutdown all tasks
        work_manager.force_shutdown_all();

        // Verify that the tasks were removed
        assert!(!work_manager.job_exists(&[1; 32]));
        assert!(!work_manager.job_exists(&[2; 32]));
    }

    #[tokio::test]
    async fn test_clear_enqueued_tasks() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote1, task1, _rx) = generate_async_protocol(1, 1, 0);
        let (remote2, task2, _rx) = generate_async_protocol(2, 2, 0);
        work_manager
            .push_task([1; 32], false, remote1, task1)
            .unwrap();
        work_manager
            .push_task([2; 32], false, remote2, task2)
            .unwrap();

        // Verify that the tasks were added
        assert!(work_manager.job_exists(&[1; 32]));
        assert!(work_manager.job_exists(&[2; 32]));

        // Clear enqueued tasks
        work_manager.clear_enqueued_tasks();

        // Verify that the tasks were removed
        assert!(!work_manager.job_exists(&[1; 32]));
        assert!(!work_manager.job_exists(&[2; 32]));
    }

    #[tokio::test]
    async fn test_max_tasks_limit() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 2, 2, PollMethod::Manual);

        let (remote1, task1, _rx) = generate_async_protocol(1, 1, 0);
        let (remote2, task2, _rx) = generate_async_protocol(2, 2, 0);
        let (remote3, task3, _rx) = generate_async_protocol(3, 3, 0);
        let (remote4, task4, _rx) = generate_async_protocol(4, 4, 0);
        let (remote5, task5, _rx) = generate_async_protocol(5, 5, 0);

        assert!(work_manager
            .push_task([1; 32], true, remote1, task1)
            .is_ok());
        assert!(work_manager
            .push_task([2; 32], true, remote2, task2)
            .is_ok());
        assert!(work_manager
            .push_task([3; 32], false, remote3, task3)
            .is_ok());
        assert!(work_manager
            .push_task([4; 32], false, remote4, task4)
            .is_ok());

        // Try to add a fifth task, should fail
        assert!(work_manager
            .push_task([5; 32], false, remote5, task5)
            .is_err());
    }

    #[tokio::test]
    async fn test_task_completion() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote1, task1, _rx) = generate_async_protocol(1, 1, 0);

        work_manager
            .push_task([1; 32], true, remote1, task1)
            .unwrap();

        // Wait some time for the tasks to finish
        tokio::time::sleep(Duration::from_millis(100)).await;
        // Poll again to cleanup
        work_manager.poll();

        // Check that the task has completed
        assert!(!work_manager.job_exists(&[1; 32]));
    }

    #[tokio::test]
    async fn test_job_removal_on_drop() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote1, task1, _rx) = generate_async_protocol(1, 1, 0);

        work_manager
            .push_task([1; 32], true, remote1, task1)
            .unwrap();

        // Manual drop of all jobs
        work_manager.force_shutdown_all();

        // Check that the job has been removed
        assert!(!work_manager.job_exists(&[1; 32]));
    }

    #[tokio::test]
    async fn test_message_delivery_to_non_existent_job() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);

        let message = TestMessage {
            message: "test".to_string(),
            associated_block_id: 0,
            associated_session_id: 1,
            associated_retry_id: 1,
            associated_task: [0; 32],
            associated_sender: 0,
            associated_recipient: None,
        };

        // Deliver a message to a non-existent job
        let delivery_type = work_manager.deliver_message(message).unwrap();

        // The message should be enqueued for future use
        assert_eq!(delivery_type, DeliveryType::EnqueuedMessage);
    }

    #[tokio::test]
    async fn test_message_delivery_to_job_with_outdated_block_id() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote1, task1, _rx) = generate_async_protocol(1, 1, 0);

        work_manager
            .push_task([0; 32], true, remote1, task1)
            .unwrap();

        let message = TestMessage {
            message: "test".to_string(),
            associated_block_id: 10, // Outdated block ID
            associated_session_id: 1,
            associated_retry_id: 1,
            associated_task: [0; 32],
            associated_sender: 0,
            associated_recipient: None,
        };

        // Try to deliver a message with an outdated block ID
        let delivery_type = work_manager.deliver_message(message).unwrap();

        // The message should be enqueued for future use
        assert_eq!(delivery_type, DeliveryType::EnqueuedMessage);
    }

    #[tokio::test]
    async fn test_can_submit_more_tasks() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 1, 0, PollMethod::Manual);
        let (remote, task, _rx) = generate_async_protocol(1, 1, 0);

        assert!(work_manager.can_submit_more_tasks());
        work_manager
            .push_task([1; 32], false, remote, task)
            .unwrap();
        assert!(!work_manager.can_submit_more_tasks());
    }

    #[tokio::test]
    async fn test_multiple_messages_single_task() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 10, 10, PollMethod::Manual);
        let (remote, task, mut rx) = generate_async_protocol(1, 1, 0);

        work_manager
            .push_task([0; 32], true, remote.clone(), task)
            .unwrap();

        let message = TestMessage {
            message: "test".to_string(),
            associated_block_id: 0,
            associated_session_id: 1,
            associated_retry_id: 1,
            associated_task: [0; 32],
            associated_sender: 0,
            associated_recipient: None,
        };

        for _ in 0..10 {
            assert_ne!(
                DeliveryType::EnqueuedMessage,
                work_manager.deliver_message(message.clone()).unwrap()
            );
        }

        for _ in 0..10 {
            let next_message = rx.recv().await.unwrap();
            assert_eq!(next_message, message);
        }
    }

    #[tokio::test]
    async fn test_task_not_started() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 1, 0, PollMethod::Manual);
        let (remote, task, _rx) = generate_async_protocol(1, 1, 0);

        work_manager
            .push_task([1; 32], false, remote, task)
            .unwrap(); // task is enqueued but not started
        assert!(!work_manager
            .get_active_sessions_metadata(0)
            .contains(&JobMetadata {
                session_id: 1,
                is_stalled: false,
                is_finished: false,
                has_started: false,
                is_active: false,
            }));
    }

    #[tokio::test]
    async fn test_can_submit_more_tasks_with_enqueued_tasks() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 1, 1, PollMethod::Manual);
        let (remote1, task1, _rx) = generate_async_protocol(1, 1, 0);
        let (remote2, task2, _rx) = generate_async_protocol(2, 2, 0);

        assert!(work_manager.can_submit_more_tasks());
        work_manager
            .push_task([1; 32], true, remote1, task1)
            .unwrap();
        assert!(work_manager.can_submit_more_tasks()); // one more task can be enqueued
        work_manager
            .push_task([2; 32], false, remote2, task2)
            .unwrap();
        assert!(!work_manager.can_submit_more_tasks()); // no more tasks can be added
    }

    #[tokio::test]
    async fn test_message_delivery_to_stalled_task() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 1, 0, PollMethod::Manual);
        let (remote, task, _rx) = generate_async_protocol(1, 1, 1); // Task will stall immediately
        let msg = TestMessage {
            message: "test".to_string(),
            associated_block_id: 0,
            associated_session_id: 1,
            associated_retry_id: 1,
            associated_task: [0; 32],
            associated_sender: 0,
            associated_recipient: None,
        };

        work_manager.push_task([0; 32], true, remote, task).unwrap();
        work_manager.poll(); // should identify and remove the stalled task

        let delivery_type = work_manager.deliver_message(msg).unwrap();
        assert_eq!(delivery_type, DeliveryType::EnqueuedMessage); // message should be enqueued because task is stalled and removed
    }

    #[tokio::test]
    async fn test_message_delivery_with_incorrect_task_hash() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 1, 0, PollMethod::Manual);
        let (remote, task, _rx) = generate_async_protocol(1, 1, 0);
        let msg = TestMessage {
            message: "test".to_string(),
            associated_block_id: 0,
            associated_session_id: 1,
            associated_retry_id: 1,
            associated_task: [1; 32], // incorrect task hash, not 0;32 as below
            associated_sender: 0,
            associated_recipient: None,
        };

        work_manager.push_task([0; 32], true, remote, task).unwrap();

        let delivery_type = work_manager.deliver_message(msg).unwrap(); // incorrect task hash
        assert_eq!(delivery_type, DeliveryType::EnqueuedMessage); // message should be enqueued because the task hash is incorrect
    }

    #[tokio::test]
    async fn test_retry_id() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 1, 0, PollMethod::Manual);
        let (remote, task, _rx) = generate_async_protocol_error(1, 0, 0);
        assert!(work_manager.latest_retry_id(&[0; 32]).is_none());
        work_manager.push_task([0; 32], true, remote, task).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(
            work_manager
                .latest_retry_id(&[0; 32])
                .expect("Should exist"),
            0
        );

        work_manager.simulate_job_stall(&[0; 32]);

        // Simulate a retry
        let (remote, task, _rx) = generate_async_protocol_error(1, 1, 0);
        work_manager.push_task([0; 32], true, remote, task).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(
            work_manager
                .latest_retry_id(&[0; 32])
                .expect("Should exist"),
            1
        );

        work_manager.simulate_job_stall(&[0; 32]);

        // Simulate that the protocol works on the third attempt
        let (remote, task, _rx) = generate_async_protocol(1, 2, 0);
        work_manager.push_task([0; 32], true, remote, task).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        // Since the protocol was a success, the retry id should be cleared
        assert!(work_manager.latest_retry_id(&[0; 32]).is_none());
    }

    #[tokio::test]
    async fn test_record_message() {
        let work_manager = ProtocolWorkManager::new(TestWorkManager, 1, 0, PollMethod::Manual);
        let (remote, task, _rx) = generate_async_protocol(1, 1, 0);
        work_manager.push_task([0; 32], true, remote, task).unwrap();
        let message = TestMessage {
            message: "test".to_string(),
            associated_block_id: 0,
            associated_session_id: 1,
            associated_retry_id: 1,
            associated_task: [0; 32],
            associated_sender: 0,
            associated_recipient: None,
        };

        work_manager.deliver_message(message.clone()).unwrap();
        let lock = work_manager.inner.read();
        let active_job = lock.active_tasks.get(&[0; 32]).unwrap();
        assert_eq!(active_job.recorded_messages.len(), 1);
        assert_eq!(active_job.recorded_messages[0], message);

        drop(lock);
        assert!(work_manager
            .visit_recorded_messages(&[0; 32], |messages| {
                assert_eq!(messages.len(), 1);
                assert_eq!(messages[0], message);
            })
            .is_some());
    }

    struct DummyRangeChecker<const N: u64>;
    #[derive(Clone)]
    struct DummyProtocolMessage;

    impl<const N: u64> ProtocolMessageMetadata<DummyRangeChecker<N>> for DummyProtocolMessage {
        fn associated_block_id(&self) -> u64 {
            0u64
        }
        fn associated_session_id(&self) -> u64 {
            0u64
        }
        fn associated_retry_id(&self) -> u64 {
            0u64
        }
        fn associated_task(&self) -> [u8; 32] {
            [0; 32]
        }

        fn associated_sender_user_id(
            &self,
        ) -> <DummyRangeChecker<N> as WorkManagerInterface>::UserID {
            0
        }

        fn associated_recipient_user_id(
            &self,
        ) -> Option<<DummyRangeChecker<N> as WorkManagerInterface>::UserID> {
            None
        }
    }

    impl<const N: u64> WorkManagerInterface for DummyRangeChecker<N> {
        type RetryID = u64;
        type UserID = u64;
        type Clock = u64;
        type ProtocolMessage = DummyProtocolMessage;
        type Error = ();
        type SessionID = u64;
        type TaskID = [u8; 32];

        fn debug(&self, _input: String) {
            todo!()
        }

        fn error(&self, _input: String) {
            todo!()
        }

        fn warn(&self, _input: String) {
            todo!()
        }

        fn clock(&self) -> Self::Clock {
            todo!()
        }

        fn acceptable_block_tolerance() -> Self::Clock {
            N
        }
    }

    #[test]
    fn test_range_above() {
        let current_block: u64 = 10;
        const TOL: u64 = 5;
        assert!(DummyRangeChecker::<TOL>::associated_block_id_acceptable(
            current_block,
            current_block
        ));
        assert!(DummyRangeChecker::<TOL>::associated_block_id_acceptable(
            current_block,
            current_block + 1
        ));
        assert!(DummyRangeChecker::<TOL>::associated_block_id_acceptable(
            current_block,
            current_block + TOL
        ));
        assert!(!DummyRangeChecker::<TOL>::associated_block_id_acceptable(
            current_block,
            current_block + TOL + 1
        ));
    }

    #[test]
    fn test_range_below() {
        let current_block: u64 = 10;
        const TOL: u64 = 5;
        assert!(DummyRangeChecker::<TOL>::associated_block_id_acceptable(
            current_block,
            current_block
        ));
        assert!(DummyRangeChecker::<TOL>::associated_block_id_acceptable(
            current_block,
            current_block - 1
        ));
        assert!(DummyRangeChecker::<TOL>::associated_block_id_acceptable(
            current_block,
            current_block - TOL
        ));
        assert!(!DummyRangeChecker::<TOL>::associated_block_id_acceptable(
            current_block,
            current_block - TOL - 1
        ));
    }
}
