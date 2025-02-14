use crate::harness::ENDOWED_TEST_NAMES;
use crate::{
    harness::{generate_env_from_node_id, TangleTestConfig},
    runner::TangleTestEnv,
    Error,
};
use futures::future::join_all;
use gadget_config::{GadgetConfiguration, Multiaddr};
use gadget_contexts::tangle::TangleClientContext;
use gadget_contexts::{keystore::KeystoreContext, tangle::TangleClient};
use gadget_core_testing_utils::runner::TestEnv;
use gadget_crypto_tangle_pair_signer::TanglePairSigner;
use gadget_event_listeners::core::InitializableEventHandler;
use gadget_keystore::backends::Backend;
use gadget_keystore::crypto::sp_core::SpSr25519;
use gadget_runners::core::error::RunnerError;
use gadget_runners::tangle::tangle::TangleConfig;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tangle_subxt::subxt::tx::Signer;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use gadget_runners::core::runner::BackgroundService;

#[derive(Clone, Debug)]
enum NodeSlot {
    Occupied(Arc<NodeHandle>),
    Empty,
}

/// Improved multi-node test environment with better control and observability
pub struct MultiNodeTestEnv {
    nodes: Arc<RwLock<Vec<NodeSlot>>>,
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
    Start {
        result_tx: oneshot::Sender<Result<(), Error>>,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum TestEvent {
    NodeAdded(usize),
    NodeRemoved(usize),
    NodeShutdown(usize),
    Error(String),
}

/// A function that returns a future that returns a result containing an event handler for the job
type EventHandlerBox = Box<dyn InitializableEventHandler + Send + 'static>;
type JobResult = Result<EventHandlerBox, RunnerError>;
type JobFuture = Pin<Box<dyn Future<Output = JobResult> + Send + 'static>>;

trait JobCreator: Fn(GadgetConfiguration) -> JobFuture + Send + Sync + 'static {}
impl<T: Fn(GadgetConfiguration) -> JobFuture + Send + Sync + 'static> JobCreator for T {}

impl MultiNodeTestEnv {
    /// Creates a new multi-node test environment
    pub async fn new<const N: usize>(config: TangleTestConfig) -> Result<Self, Error> {
        const { assert!(N > 0, "Must have at least 1 initial node") };

        let (command_tx, command_rx) = mpsc::channel(32);
        let (event_tx, _) = broadcast::channel(100);
        let (initialized_tx, _initialized_rx) = oneshot::channel();

        let env = Self {
            nodes: Arc::new(RwLock::new(vec![NodeSlot::Empty; N])),
            command_tx,
            event_tx: event_tx.clone(),
            config: Arc::new(config),
            initialized_tx: Some(initialized_tx),
            running_nodes: Arc::new(AtomicUsize::new(0)),
        };

        Self::spawn_command_handler(
            env.nodes.clone(),
            env.config.clone(),
            env.running_nodes.clone(),
            command_rx,
            event_tx,
        );

        Ok(env)
    }

    /// Initializes the multi node test environment with N nodes
    pub async fn initialize(&mut self) -> Result<(), Error> {
        if self.initialized_tx.is_none() {
            // Already initialized
            return Ok(());
        }

        let initial_node_count = self.nodes.read().await.len();

        // First add N nodes
        for node_id in 0..initial_node_count {
            self.add_node(node_id).await?;
        }

        // Setup the bootnodes
        if initial_node_count > 1 {
            let nodes = self.nodes.read().await;
            for (index, node) in nodes.iter().enumerate() {
                let NodeSlot::Occupied(node) = node else {
                    panic!("Not all nodes were initialized");
                };

                let mut bootnodes = Vec::new();
                for node in nodes.iter().enumerate().filter(|(n, _)| *n != index) {
                    let NodeSlot::Occupied(node) = node.1 else {
                        panic!("Not all nodes were initialized");
                    };

                    bootnodes.push(node.addr.clone());
                }

                let mut env = node.test_env.write().await;
                env.update_networking_config(bootnodes, node.port);
            }
        }

        // Signal initialization is complete
        if let Some(tx) = self.initialized_tx.take() {
            let _ = tx.send(());
        }

        Ok(())
    }

    /// Adds a job to the node to be executed when the test is run.
    ///
    /// The job is added to the end of the list of jobs and can be stopped using the `stop_job`
    /// method.
    pub async fn add_job<
        T: Fn(GadgetConfiguration) -> F + Clone + Send + Sync + 'static,
        F: Future<Output = Result<K, E>> + Send + 'static,
        K: InitializableEventHandler + Send + Sync + 'static,
        E: std::fmt::Debug + Send + 'static,
    >(&self, creator: T) -> Result<(), E> {
        let mut nodes = self.nodes.write().await;
        for node in nodes.iter_mut() {
            if let NodeSlot::Occupied(node) = node {
                let job = creator(node.gadget_config().await).await?;
                node.add_job(job).await;
            }
        }

        Ok(())
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        // Start all nodes' runners
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(EnvironmentCommand::Start { result_tx })
            .await
            .map_err(|e| Error::Setup(e.to_string()))?;

        // Wait for initialization to complete
        result_rx.await.map_err(|e| Error::Setup(e.to_string()))??;

        Ok(())
    }

    /// Adds a new node to the test environment
    async fn add_node(&self, node_id: usize) -> Result<(), Error> {
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
        nodes: Arc<RwLock<Vec<NodeSlot>>>,
        config: Arc<TangleTestConfig>,
        running_nodes: Arc<AtomicUsize>,
        mut command_rx: mpsc::Receiver<EnvironmentCommand>,
        event_tx: broadcast::Sender<TestEvent>,
    ) {
        tokio::spawn(async move {
            let nodes = nodes.clone();
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    EnvironmentCommand::AddNode { node_id, result_tx } => {
                        let result = Self::handle_add_node(
                            nodes.clone(),
                            node_id,
                            config.clone(),
                            &event_tx,
                        )
                        .await;
                        let _ = result_tx.send(result);
                    }
                    EnvironmentCommand::RemoveNode { node_id, result_tx } => {
                        let result =
                            Self::handle_remove_node(nodes.clone(), node_id, &event_tx).await;
                        let _ = result_tx.send(result);
                    }
                    EnvironmentCommand::Start { result_tx } => {
                        let result =
                            Self::handle_start(nodes.clone(), &event_tx, running_nodes.clone())
                                .await;
                        let _ = result_tx.send(result);
                    }
                    EnvironmentCommand::Shutdown => {
                        Self::handle_shutdown(nodes.clone(), &event_tx).await;
                        break;
                    }
                }
            }
        });
    }

    pub async fn node_handles(&self) -> Vec<Arc<NodeHandle>> {
        self.nodes
            .read()
            .await
            .iter()
            .filter_map(|n| match n {
                NodeSlot::Occupied(node) => Some(node.clone()),
                NodeSlot::Empty => None,
            })
            .collect()
    }

    async fn handle_add_node(
        nodes: Arc<RwLock<Vec<NodeSlot>>>,
        node_id: usize,
        config: Arc<TangleTestConfig>,
        event_tx: &broadcast::Sender<TestEvent>,
    ) -> Result<(), Error> {
        let node = NodeHandle::new(node_id, &config).await?;
        nodes.write().await[node_id] = NodeSlot::Occupied(node);
        let _ = event_tx.send(TestEvent::NodeAdded(node_id));
        Ok(())
    }

    async fn handle_remove_node(
        nodes: Arc<RwLock<Vec<NodeSlot>>>,
        node_id: usize,
        event_tx: &broadcast::Sender<TestEvent>,
    ) -> Result<(), Error> {
        let nodes = nodes.read().await;

        let NodeSlot::Occupied(node) = nodes[node_id].clone() else {
            return Err(Error::Setup(format!("Node {} not found", node_id)));
        };

        // Send shutdown command to the node
        if let Err(e) = node.shutdown().await {
            let _ = event_tx.send(TestEvent::Error(format!(
                "Failed to shutdown node {}: {}",
                node_id, e
            )));
        }
        let _ = event_tx.send(TestEvent::NodeRemoved(node_id));
        Ok(())
    }

    async fn handle_start(
        nodes: Arc<RwLock<Vec<NodeSlot>>>,
        event_tx: &broadcast::Sender<TestEvent>,
        running_nodes: Arc<AtomicUsize>,
    ) -> Result<(), Error> {
        let nodes = nodes.read().await;

        assert!(
            nodes.iter().all(|n| matches!(n, NodeSlot::Occupied(_))),
            "Not all nodes were initialized"
        );

        // Start all node runners concurrently
        let futures = nodes.iter().enumerate().map(|(node_id, node)| {
            let running_nodes = running_nodes.clone();

            async move {
                let NodeSlot::Occupied(node) = node else {
                    unreachable!()
                };

                if let Err(e) = node.start_runner().await {
                    let _ = event_tx.send(TestEvent::Error(format!(
                        "Failed to start node {}: {}",
                        node_id, e
                    )));
                    return Err(e);
                }
                running_nodes.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        // Wait for all nodes to start
        let results = join_all(futures).await;
        for result in results {
            result?;
        }

        Ok(())
    }

    async fn handle_shutdown(
        nodes: Arc<RwLock<Vec<NodeSlot>>>,
        event_tx: &broadcast::Sender<TestEvent>,
    ) {
        let nodes = nodes.read().await;
        for (node_id, node) in nodes.iter().enumerate() {
            if let NodeSlot::Occupied(node) = node {
                if let Err(e) = node.shutdown().await {
                    let _ = event_tx.send(TestEvent::Error(format!(
                        "Failed to shutdown node {}: {}",
                        node_id, e
                    )));
                }
            }
        }
    }
}

struct NodeState {
    is_running: bool,
}

impl Debug for NodeState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeState")
            .field("is_running", &self.is_running)
            .finish()
    }
}

/// Commands that can be sent to individual nodes
enum NodeCommand {
    StartRunner {
        result_tx: oneshot::Sender<Result<(), Error>>,
    },
    Shutdown,
}

impl Debug for NodeCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeCommand::StartRunner { .. } => f.write_str("StartRunner"),
            NodeCommand::Shutdown => f.write_str("Shutdown"),
        }
    }
}

/// Represents a single node in the multi-node test environment
pub struct NodeHandle {
    node_id: usize,
    addr: Multiaddr,
    port: u16,
    client: TangleClient,
    signer: TanglePairSigner<sp_core::sr25519::Pair>,
    state: Arc<RwLock<NodeState>>,
    command_tx: mpsc::Sender<NodeCommand>,
    test_env: Arc<RwLock<TangleTestEnv>>,
}

impl NodeHandle {
    /// Adds a job to the node to be executed when the test is run.
    ///
    /// The job is added to the end of the list of jobs and can be stopped using the `stop_job`
    /// method.
    pub async fn add_job<K: InitializableEventHandler + Send + Sync + 'static>(&self, job: K) {
        self.test_env.write().await.add_job(job)
    }

    pub async fn add_background_service<K: BackgroundService + Send + 'static>(&self, service: K) {
        self.test_env.write().await.add_background_service(service)
    }

    pub async fn gadget_config(&self) -> GadgetConfiguration {
        self.test_env.read().await.get_gadget_config()
    }
}

impl Debug for NodeHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeHandle")
            .field("node_id", &self.node_id)
            .field("signer", &self.signer.address())
            .field("test_env", &self.test_env)
            .finish()
    }
}

// Implementation for NodeHandle
impl NodeHandle {
    async fn new(node_id: usize, config: &TangleTestConfig) -> Result<Arc<Self>, Error> {
        let (command_tx, command_rx) = mpsc::channel(32);
        let state = Arc::new(RwLock::new(NodeState { is_running: true }));

        // Create node environment and client
        let env = generate_env_from_node_id(
            ENDOWED_TEST_NAMES[node_id],
            config.http_endpoint.clone(),
            config.ws_endpoint.clone(),
            config.temp_dir.as_path(),
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

        let port = find_open_tcp_bind_port();
        gadget_logging::info!("Binding node {node_id} to port {port}");

        let addr = Multiaddr::from_str(&format!("/ip4/127.0.0.1/tcp/{port}"))
            .expect("Should parse MultiAddr");

        let node = Arc::new(Self {
            node_id,
            addr,
            port,
            client,
            signer: sr25519_signer,
            state,
            command_tx,
            test_env: Arc::new(RwLock::new(test_env)),
        });

        Self::spawn_command_handler(node.clone(), command_rx);
        Ok(node)
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

    fn spawn_command_handler(node: Arc<Self>, mut command_rx: mpsc::Receiver<NodeCommand>) {
        let state = node.state.clone();

        tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    NodeCommand::StartRunner { result_tx } => {
                        let result = node.test_env.read().await.run_runner().await;
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

pub fn find_open_tcp_bind_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("Should bind to localhost");
    let port = listener
        .local_addr()
        .expect("Should have a local address")
        .port();
    drop(listener);
    port
}
