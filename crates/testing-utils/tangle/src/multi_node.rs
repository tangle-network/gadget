use crate::{
    harness::{generate_env_from_node_id, TangleTestConfig},
    node::transactions::submit_and_verify_job,
    runner::TangleTestEnv,
    Error, InputValue, OutputValue,
};
use gadget_config::GadgetConfiguration;
use gadget_contexts::{keystore::KeystoreContext, tangle::TangleClient};
use gadget_crypto_tangle_pair_signer::TanglePairSigner;
use gadget_event_listeners::core::InitializableEventHandler;
use gadget_keystore::crypto::sp_core::SpSr25519;
use gadget_runners::core::error::RunnerError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{collections::HashMap, sync::Arc};
use subxt::ext::futures::future::join_all;
use tangle_subxt::tangle_testnet_runtime::api::services::events::JobResultSubmitted;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};

/// Represents a single node in the multi-node test environment
pub struct NodeHandle {
    node_id: usize,
    client: TangleClient,
    signer: TanglePairSigner<sp_core::sr25519::Pair>,
    state: Arc<RwLock<NodeState>>,
    command_tx: mpsc::Sender<NodeCommand>,
    test_env: TangleTestEnv,
}

#[derive(Debug)]
struct NodeState {
    is_running: bool,
    jobs: HashMap<u64, Box<dyn InitializableEventHandler>>,
}

/// Commands that can be sent to individual nodes
#[derive(Debug)]
enum NodeCommand {
    AddJob {
        job: Box<dyn InitializableEventHandler>,
    },
    StartRunner {
        result_tx: oneshot::Sender<Result<(), Error>>,
    },
    Shutdown,
}

/// Improved multi-node test environment with better control and observability
pub struct MultiNodeTestEnv {
    nodes: Arc<RwLock<HashMap<usize, NodeHandle>>>,
    command_tx: mpsc::Sender<EnvironmentCommand>,
    event_tx: broadcast::Sender<TestEvent>,
    config: Arc<TangleTestConfig>,
    initialized_tx: Option<oneshot::Sender<()>>,
    running_nodes: Arc<AtomicUsize>,
}

#[derive(Debug)]
enum EnvironmentCommand {
    AddNode {
        node_id: usize,
        result_tx: oneshot::Sender<Result<(), Error>>,
    },
    RemoveNode {
        node_id: usize,
        result_tx: oneshot::Sender<Result<(), Error>>,
    },
    Initialize {
        result_tx: oneshot::Sender<Result<(), Error>>,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum TestEvent {
    NodeAdded(usize),
    NodeRemoved(usize),
    JobAdded { node_id: usize, job_id: u64 },
    NodeShutdown(usize),
    Error(String),
}

impl MultiNodeTestEnv {
    /// Creates a new multi-node test environment
    pub async fn new(config: TangleTestConfig) -> Result<Self, Error> {
        let (command_tx, command_rx) = mpsc::channel(32);
        let (event_tx, _) = broadcast::channel(100);
        let (initialized_tx, initialized_rx) = oneshot::channel();

        let env = Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            command_tx,
            event_tx: event_tx.clone(),
            config: Arc::new(config),
            initialized_tx: Some(initialized_tx),
            running_nodes: Arc::new(AtomicUsize::new(0)),
        };

        env.spawn_command_handler(command_rx, event_tx);
        Ok(env)
    }

    /// Initializes the multi node test environment with N nodes
    pub async fn initialize<const N: usize>(&mut self) -> Result<(), Error> {
        // First add N nodes
        for node_id in 0..N {
            self.add_node(node_id).await?;
        }

        // Then start all nodes' runners
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(EnvironmentCommand::Initialize { result_tx })
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;

        // Wait for initialization to complete
        result_rx.await.map_err(|e| Error::Setup(e.to_string()))?;

        // Signal initialization is complete
        if let Some(tx) = self.initialized_tx.take() {
            let _ = tx.send(());
        }

        Ok(())
    }

    /// Adds a new node to the test environment
    pub async fn add_node(&self, node_id: usize) -> Result<(), Error> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(EnvironmentCommand::AddNode { node_id, result_tx })
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;
        result_rx.await.map_err(|e| Error::Setup(e.to_string()))?
    }

    /// Subscribes to test environment events
    pub fn subscribe(&self) -> broadcast::Receiver<TestEvent> {
        self.event_tx.subscribe()
    }

    /// Removes a node from the test environment
    pub async fn remove_node(&self, node_id: usize) -> Result<(), Error> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(EnvironmentCommand::RemoveNode { node_id, result_tx })
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;
        result_rx.await.map_err(|e| Error::Setup(e.to_string()))?
    }

    /// Shuts down the test environment
    pub async fn shutdown(&self) -> Result<(), Error> {
        self.command_tx
            .send(EnvironmentCommand::Shutdown)
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;
        Ok(())
    }

    fn spawn_command_handler(
        &self,
        mut command_rx: mpsc::Receiver<EnvironmentCommand>,
        event_tx: broadcast::Sender<TestEvent>,
    ) {
        let nodes = self.nodes.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    EnvironmentCommand::AddNode { node_id, result_tx } => {
                        let result =
                            Self::handle_add_node(&nodes, node_id, &config, &event_tx).await;
                        let _ = result_tx.send(result);
                    }
                    EnvironmentCommand::RemoveNode { node_id, result_tx } => {
                        let result = Self::handle_remove_node(&nodes, node_id, &event_tx).await;
                        let _ = result_tx.send(result);
                    }
                    EnvironmentCommand::Initialize { result_tx } => {
                        let result =
                            Self::handle_initialize(&nodes, &event_tx, self.running_nodes.clone())
                                .await;
                        let _ = result_tx.send(result);
                    }
                    EnvironmentCommand::Shutdown => {
                        Self::handle_shutdown(&nodes, &event_tx).await;
                        break;
                    }
                }
            }
        });
    }

    async fn handle_add_node(
        nodes: &Arc<RwLock<HashMap<usize, NodeHandle>>>,
        node_id: usize,
        config: &TangleTestConfig,
        event_tx: &broadcast::Sender<TestEvent>,
    ) -> Result<(), Error> {
        let node = NodeHandle::new(node_id, config).await?;
        nodes.write().await.insert(node_id, node);
        let _ = event_tx.send(TestEvent::NodeAdded(node_id));
        Ok(())
    }

    async fn handle_remove_node(
        nodes: &Arc<RwLock<HashMap<usize, NodeHandle>>>,
        node_id: usize,
        event_tx: &broadcast::Sender<TestEvent>,
    ) -> Result<(), Error> {
        let mut nodes = nodes.write().await;
        if let Some(node) = nodes.remove(&node_id) {
            // Send shutdown command to the node
            if let Err(e) = node.shutdown().await {
                let _ = event_tx.send(TestEvent::Error(format!(
                    "Failed to shutdown node {}: {}",
                    node_id, e
                )));
            }
            let _ = event_tx.send(TestEvent::NodeRemoved(node_id));
            Ok(())
        } else {
            Err(Error::Setup(format!("Node {} not found", node_id)))
        }
    }

    async fn handle_initialize(
        nodes: &Arc<RwLock<HashMap<usize, NodeHandle>>>,
        event_tx: &broadcast::Sender<TestEvent>,
        running_nodes: Arc<AtomicUsize>,
    ) -> Result<(), Error> {
        let nodes = nodes.read().await;

        // Start all node runners concurrently
        let futures = nodes.iter().map(|(node_id, node)| async move {
            if let Err(e) = node.start_runner().await {
                let _ = event_tx.send(TestEvent::Error(format!(
                    "Failed to start node {}: {}",
                    node_id, e
                )));
                return Err(e);
            }
            running_nodes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        // Wait for all nodes to start
        let results = join_all(futures).await;
        for result in results {
            result?;
        }

        Ok(())
    }

    async fn handle_shutdown(
        nodes: &Arc<RwLock<HashMap<usize, NodeHandle>>>,
        event_tx: &broadcast::Sender<TestEvent>,
    ) {
        let mut nodes = nodes.write().await;
        for (node_id, node) in nodes.drain() {
            if let Err(e) = node.shutdown().await {
                let _ = event_tx.send(TestEvent::Error(format!(
                    "Failed to shutdown node {}: {}",
                    node_id, e
                )));
            }
        }
    }

    /// Gets a reference to a specific node handle
    pub async fn get_node(&self, node_id: usize) -> Result<Arc<NodeHandle>, Error> {
        let nodes = self.nodes.read().await;
        nodes
            .get(&node_id)
            .cloned()
            .map(Arc::new)
            .ok_or_else(|| Error::Setup(format!("Node {} not found", node_id)))
    }

    /// Gets references to all node handles
    pub async fn get_all_nodes(&self) -> Vec<Arc<NodeHandle>> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().map(Arc::new).collect()
    }
}

// Implementation for NodeHandle
impl NodeHandle {
    async fn new(node_id: usize, config: &TangleTestConfig) -> Result<Self, Error> {
        let (command_tx, command_rx) = mpsc::channel(32);
        let state = Arc::new(RwLock::new(NodeState {
            is_running: true,
            jobs: HashMap::new(),
        }));

        // Create node environment and client
        let env = generate_env_from_node_id(
            node_id,
            config.http_endpoint.clone()?,
            config.ws_endpoint.clone()?,
        )
        .await?;

        let client = env.tangle_client().await?;
        let keystore = env.keystore();
        let sr25519_public = keystore
            .first_local::<SpSr25519>()
            .map_err(|err| RunnerError::Other(err.to_string()))?;
        let sr25519_pair = keystore
            .get_secret::<SpSr25519>(&sr25519_public)
            .map_err(|err| RunnerError::Other(err.to_string()))?;
        let sr25519_signer = TanglePairSigner::new(sr25519_pair.0);

        // Create TangleTestEnv for this node
        let test_env = TangleTestEnv::new(TangleConfig::default(), env.clone())?;

        let node = Self {
            node_id,
            client,
            signer: sr25519_signer,
            state,
            command_tx,
            test_env,
        };

        node.spawn_command_handler(command_rx);
        Ok(node)
    }

    /// Adds a job to the node
    pub async fn add_job(&self, job: Box<dyn InitializableEventHandler>) -> Result<(), Error> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(NodeCommand::AddJob { job })
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;
        result_rx.await.map_err(|e| Error::Setup(e.to_string()))?
    }

    /// Shuts down the node
    pub async fn shutdown(&self) -> Result<(), Error> {
        self.command_tx
            .send(NodeCommand::Shutdown)
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;

        // Wait for the node to mark itself as not running
        let mut retries = 0;
        while retries < 10 {
            if !self.state.read().await.is_running {
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            retries += 1;
        }

        Err(Error::Setup("Node failed to shutdown in time".to_string()))
    }

    async fn start_runner(&self) -> Result<(), Error> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(NodeCommand::StartRunner { result_tx })
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;
        result_rx.await.map_err(|e| Error::Setup(e.to_string()))?
    }

    fn spawn_command_handler(&self, mut command_rx: mpsc::Receiver<NodeCommand>) {
        let state = self.state.clone();
        let test_env = self.test_env.clone();

        tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    NodeCommand::AddJob { job } => {
                        let mut state = state.write().await;
                        let job_id = state.jobs.len() as u64;
                        state.jobs.insert(job_id, job);
                    }
                    NodeCommand::StartRunner { result_tx } => {
                        let result = test_env.run_runner().await;
                        let _ = result_tx.send(result.map_err(|e| Error::Setup(e.to_string())));
                    }
                    NodeCommand::Shutdown => {
                        let mut state = state.write().await;
                        state.is_running = false;
                        break;
                    }
                }
            }
        });
    }

    /// Gets a reference to the node's client
    pub fn client(&self) -> &TangleClient {
        &self.client
    }

    /// Gets a reference to the node's signer
    pub fn signer(&self) -> &TanglePairSigner<sp_core::sr25519::Pair> {
        &self.signer
    }
}
