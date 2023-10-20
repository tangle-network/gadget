use parking_lot::RwLock;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::{Hash, Hasher},
    pin::Pin,
    sync::Arc,
};
use sync_wrapper::SyncWrapper;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PollMethod {
    Interval { millis: u64 },
    Manual,
}
pub struct WorkManager<WM: WorkManagerInterface> {
    inner: Arc<RwLock<WorkManagerInner<WM>>>,
    utility: Arc<WM>,
    // for now, use a hard-coded value for the number of tasks
    max_tasks: Arc<usize>,
    max_enqueued_tasks: Arc<usize>,
    poll_method: Arc<PollMethod>,
    to_handler: tokio::sync::mpsc::UnboundedSender<[u8; 32]>,
}

impl<WM: WorkManagerInterface> Clone for WorkManager<WM> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            utility: self.utility.clone(),
            max_tasks: self.max_tasks.clone(),
            max_enqueued_tasks: self.max_enqueued_tasks.clone(),
            poll_method: self.poll_method.clone(),
            to_handler: self.to_handler.clone(),
        }
    }
}

pub struct WorkManagerInner<WM: WorkManagerInterface> {
    pub active_tasks: HashSet<Job<WM>>,
    pub enqueued_tasks: VecDeque<Job<WM>>,
    // task hash => SSID => enqueued messages
    pub enqueued_messages: EnqueuedMessage<[u8; 32], WM::SSID, WM::ProtocolMessage>,
}

pub type EnqueuedMessage<A, B, C> = HashMap<A, HashMap<B, VecDeque<C>>>;

pub trait WorkManagerInterface: Send + Sync + 'static + Sized {
    type SSID: Copy + Hash + Eq + PartialEq + Send + Sync + 'static;
    type Clock: Copy + Debug + Eq + PartialEq + Send + Sync + 'static;
    type ProtocolMessage: ProtocolMessageMetadata<Self> + Send + Sync + 'static;
    type Error: Debug + Send + Sync + 'static;
    type SessionID: Copy + Hash + Eq + PartialEq + Display + Debug + Send + Sync + 'static;
    fn debug(&self, input: String);
    fn error(&self, input: String);
    fn warn(&self, input: String);
    fn clock(&self) -> Self::Clock;
    fn associated_block_id_acceptable(now: Self::Clock, compare: Self::Clock) -> bool;
}

pub trait ProtocolMessageMetadata<WM: WorkManagerInterface> {
    fn associated_block_id(&self) -> WM::Clock;
    fn associated_session_id(&self) -> WM::SessionID;
    fn associated_ssid(&self) -> WM::SSID;
}

pub trait ProtocolRemote<WM: WorkManagerInterface>: Send + Sync + 'static {
    fn start(&self) -> Result<(), WM::Error>;
    fn session_id(&self) -> WM::SessionID;
    fn set_as_primary(&self);
    fn has_stalled(&self, now: WM::Clock) -> bool;
    fn started_at(&self) -> WM::Clock;
    fn shutdown(&self, reason: ShutdownReason) -> Result<(), WM::Error>;
    fn is_done(&self) -> bool;
    fn deliver_message(&self, message: WM::ProtocolMessage) -> Result<(), WM::Error>;
    fn has_started(&self) -> bool;
    fn is_active(&self) -> bool;
    fn ssid(&self) -> WM::SSID;
}

#[derive(Debug)]
pub struct JobMetadata<WM: WorkManagerInterface> {
    pub session_id: WM::SessionID,
    pub is_stalled: bool,
    pub is_finished: bool,
    pub has_started: bool,
    pub is_active: bool,
}

impl<WM: WorkManagerInterface> WorkManager<WM> {
    pub fn new(
        utility: WM,
        max_tasks: usize,
        max_enqueued_tasks: usize,
        poll_method: PollMethod,
    ) -> Self {
        let (to_handler, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let this = Self {
            inner: Arc::new(RwLock::new(WorkManagerInner {
                active_tasks: HashSet::new(),
                enqueued_tasks: VecDeque::new(),
                enqueued_messages: HashMap::new(),
            })),
            utility: Arc::new(utility),
            max_tasks: Arc::new(max_tasks),
            max_enqueued_tasks: Arc::new(max_enqueued_tasks),
            to_handler,
            poll_method: Arc::new(poll_method),
        };

        if let PollMethod::Interval { millis } = poll_method {
            let this_worker = this.clone();
            let handler = async move {
                let job_receiver_worker = this_worker.clone();
                let logger = job_receiver_worker.utility.clone();

                let job_receiver = async move {
                    while let Some(task_hash) = rx.recv().await {
                        job_receiver_worker
                            .utility
                            .debug(format!("[worker] Received job {task_hash:?}",));
                        job_receiver_worker.poll();
                    }
                };

                let periodic_poller = async move {
                    let mut interval =
                        tokio::time::interval(std::time::Duration::from_millis(millis));
                    loop {
                        interval.tick().await;
                        this_worker.poll();
                    }
                };

                tokio::select! {
                    _ = job_receiver => {
                        logger.error("[worker] job_receiver exited".to_string());
                    },
                    _ = periodic_poller => {
                        logger.error("[worker] periodic_poller exited".to_string());
                    }
                }
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
    pub fn push_task(
        &self,
        task_hash: [u8; 32],
        force_start: bool,
        handle: Arc<dyn ProtocolRemote<WM>>,
        task: Pin<Box<dyn SendFuture<'static, ()>>>,
    ) -> Result<(), WorkManagerError> {
        let mut lock = self.inner.write();
        // set as primary, that way on drop, the async protocol ends
        handle.set_as_primary();
        let job = Job {
            task: Arc::new(RwLock::new(Some(task.into()))),
            handle,
            task_hash,
            utility: self.utility.clone(),
        };

        if force_start {
            // This job has priority over the max_tasks limit
            self.utility.debug(format!(
                "[FORCE START] Force starting task {}",
                hex::encode(task_hash)
            ));
            self.start_job_unconditional(job, &mut *lock);
            return Ok(());
        }

        lock.enqueued_tasks.push_back(job);

        if *self.poll_method != PollMethod::Manual {
            self.to_handler
                .send(task_hash)
                .map_err(|_| WorkManagerError::PushTaskFailed {
                    reason: "Failed to send job to worker".to_string(),
                })
        } else {
            Ok(())
        }
    }

    pub fn can_submit_more_tasks(&self) -> bool {
        let lock = self.inner.read();
        lock.enqueued_tasks.len() < *self.max_enqueued_tasks
    }

    // Only relevant for keygen
    pub fn get_active_sessions_metadata(&self, now: WM::Clock) -> Vec<JobMetadata<WM>> {
        self.inner
            .read()
            .active_tasks
            .iter()
            .map(|r| r.metadata(now))
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
        lock.active_tasks.retain(|job| {
            let is_stalled = job.handle.has_stalled(now);
            if is_stalled {
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
                self.start_job_unconditional(job, &mut *lock);
            } else {
                break;
            }
        }

        // Next, remove any outdated enqueued messages to prevent RAM bloat
        let mut to_remove = vec![];
        for (hash, queue) in lock.enqueued_messages.iter_mut() {
            for (ssid, queue) in queue.iter_mut() {
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
                    to_remove.push((*hash, *ssid));
                }
            }
        }

        // Next, to prevent the existence of piling-up empty *inner* queues, remove them
        for (hash, ssid) in to_remove {
            lock.enqueued_messages
                .get_mut(&hash)
                .expect("Should be available")
                .remove(&ssid);
        }

        // Finally, remove any empty outer maps
        lock.enqueued_messages.retain(|_, v| !v.is_empty());
    }

    fn start_job_unconditional(&self, job: Job<WM>, lock: &mut WorkManagerInner<WM>) {
        self.utility.debug(format!(
            "[worker] Starting job {:?}",
            hex::encode(job.task_hash)
        ));
        if let Err(err) = job.handle.start() {
            self.utility.error(format!(
                "Failed to start job {:?}: {err:?}",
                hex::encode(job.task_hash)
            ));
        } else {
            // deliver all the enqueued messages to the protocol now
            if let Some(mut enqueued_messages_map) = lock.enqueued_messages.remove(&job.task_hash) {
                let job_ssid = job.handle.ssid();
                if let Some(mut enqueued_messages) = enqueued_messages_map.remove(&job_ssid) {
                    self.utility.debug(format!(
                        "Will now deliver {} enqueued message(s) to the async protocol for {:?}",
                        enqueued_messages.len(),
                        hex::encode(job.task_hash)
                    ));

                    while let Some(message) = enqueued_messages.pop_front() {
                        if should_deliver(&job, &message, job.task_hash) {
                            if let Err(err) = job.handle.deliver_message(message) {
                                self.utility.error(format!(
                                    "Unable to deliver message for job {:?}: {err:?}",
                                    hex::encode(job.task_hash)
                                ));
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
        lock.active_tasks.insert(job);
        // run the task
        let task = async move {
            let task = task.write().take().expect("Should not happen");
            task.into_inner().await
        };

        // Spawn the task. When it finishes, it will clean itself up
        tokio::task::spawn(task);
    }

    pub fn job_exists(&self, job: &[u8; 32]) -> bool {
        let lock = self.inner.read();
        lock.active_tasks.contains(job) || lock.enqueued_tasks.iter().any(|j| &j.task_hash == job)
    }

    pub fn deliver_message(&self, msg: WM::ProtocolMessage, message_task_hash: [u8; 32]) {
        self.utility.debug(format!(
            "Delivered message is intended for session_id = {}",
            msg.associated_session_id()
        ));
        let mut lock = self.inner.write();

        // check the enqueued
        for task in lock.enqueued_tasks.iter() {
            if should_deliver(task, &msg, message_task_hash) {
                self.utility.debug(format!(
                    "Message is for this ENQUEUED signing execution in session: {}",
                    task.handle.session_id()
                ));
                if let Err(_err) = task.handle.deliver_message(msg) {
                    self.utility
                        .warn("Failed to deliver message to signing task".to_string());
                }

                return;
            }
        }

        // check the currently signing
        for task in lock.active_tasks.iter() {
            if should_deliver(task, &msg, message_task_hash) {
                self.utility.debug(format!(
                    "Message is for this signing CURRENT execution in session: {}",
                    task.handle.session_id()
                ));
                if let Err(err) = task.handle.deliver_message(msg) {
                    self.utility.warn(format!(
                        "Failed to deliver message to signing task: {err:?}"
                    ));
                }

                return;
            }
        }

        // if the protocol is neither started nor enqueued, then, this message may be for a future
        // async protocol. Store the message
        let current_running_session_ids: Vec<WM::SessionID> = lock
            .active_tasks
            .iter()
            .map(|job| job.handle.session_id())
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
            .entry(msg.associated_ssid())
            .or_default()
            .push_back(msg)
    }
}

pub struct Job<WM: WorkManagerInterface> {
    task_hash: [u8; 32],
    utility: Arc<WM>,
    handle: Arc<dyn ProtocolRemote<WM>>,
    task: Arc<RwLock<Option<SyncFuture<()>>>>,
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
}

pub enum ShutdownReason {
    Stalled,
    DropCode,
}

pub trait SendFuture<'a, T>: Send + Future<Output = T> + 'a {}
impl<'a, F: Send + Future<Output = T> + 'a, T> SendFuture<'a, T> for F {}

pub type SyncFuture<T> = SyncWrapper<Pin<Box<dyn SendFuture<'static, T>>>>;

impl<WM: WorkManagerInterface> std::borrow::Borrow<[u8; 32]> for Job<WM> {
    fn borrow(&self) -> &[u8; 32] {
        &self.task_hash
    }
}

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
            "Will remove job {:?} from currently_signing_proposals",
            hex::encode(self.task_hash)
        ));
        let _ = self.handle.shutdown(ShutdownReason::DropCode);
    }
}

fn should_deliver<WM: WorkManagerInterface>(
    task: &Job<WM>,
    msg: &WM::ProtocolMessage,
    message_task_hash: [u8; 32],
) -> bool {
    task.handle.session_id() == msg.associated_session_id()
        && task.task_hash == message_task_hash
        && task.handle.ssid() == msg.associated_ssid()
        && WM::associated_block_id_acceptable(
            task.handle.started_at(), // use to be associated_block_id
            msg.associated_block_id(),
        )
}
