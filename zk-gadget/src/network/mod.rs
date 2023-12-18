use async_trait::async_trait;
use bytes::Bytes;
use futures_util::sink::SinkExt;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::StreamExt;
use gadget_core::job_manager::WorkManagerInterface;
use mpc_net::multi::WrappedStream;
use mpc_net::prod::{CertToDer, RustlsCertificate};
use mpc_net::MpcNetError;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio_rustls::rustls::server::NoClientAuth;
use tokio_rustls::rustls::{RootCertStore, ServerConfig};
use tokio_rustls::{rustls, TlsAcceptor, TlsStream};

/// Type should correspond to the on-chain identifier of the registrant
pub type RegistantId = sp_core::ecdsa::Public;

// TODO: ZK setup related hashmaps need cleanups for tasks that the local node never participates in

#[derive(Clone)]
pub enum ZkNetworkService {
    King {
        listener: Arc<Mutex<Option<tokio::net::TcpListener>>>,
        registrants: Arc<Mutex<HashMap<RegistantId, Registrant>>>,
        to_gadget: UnboundedSender<RegistryPacket>,
        to_outbound_txs: Arc<RwLock<HashMap<RegistantId, UnboundedSender<RegistryPacket>>>>,
        inbound_messages: Arc<Mutex<UnboundedReceiver<RegistryPacket>>>,
        zk_setup: Arc<tokio::sync::RwLock<HashMap<[u8; 32], Arc<Mutex<KingRegistryResult>>>>>,
        identity: RustlsCertificate,
        registry_id: RegistantId,
    },
    Client {
        king_registry_addr: SocketAddr,
        king_registry_id: Option<RegistantId>,
        registry_id: RegistantId,
        cert_der: Vec<u8>,
        local_to_outbound_tx: UnboundedSender<RegistryPacket>,
        inbound_messages: Arc<Mutex<UnboundedReceiver<RegistryPacket>>>,
        // A mapping of task IDs and associated list of registry IDs
        registry_map: Arc<Mutex<HashMap<[u8; 32], ClientRegistryResult>>>,
    },
}

pub struct ClientRegistryResult {
    tx: Option<tokio::sync::oneshot::Sender<HashMap<u32, RegistantId>>>,
    rx: Option<tokio::sync::oneshot::Receiver<HashMap<u32, RegistantId>>>,
}

pub struct KingRegistryResult {
    tx: Option<UnboundedSender<ZkSetupPacket>>,
    rx: Option<UnboundedReceiver<ZkSetupPacket>>,
}

#[allow(dead_code)]
pub struct Registrant {
    id: RegistantId,
    cert_der: Vec<u8>,
}

use crate::Error;
use gadget_common::gadget::message::GadgetProtocolMessage;
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WebbWorkManager;

pub fn create_server_tls_acceptor<T: CertToDer>(
    server_certificate: T,
) -> Result<TlsAcceptor, MpcNetError> {
    let client_auth = NoClientAuth::boxed();
    let server_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_client_cert_verifier(client_auth)
        .with_single_cert(
            vec![rustls::Certificate(
                server_certificate.serialize_certificate_to_der()?,
            )],
            rustls::PrivateKey(server_certificate.serialize_private_key_to_der()?),
        )
        .unwrap();
    Ok(TlsAcceptor::from(Arc::new(server_config)))
}

impl ZkNetworkService {
    pub async fn new_king<T: std::net::ToSocketAddrs>(
        registry_id: RegistantId,
        bind_addr: T,
        identity: RustlsCertificate,
    ) -> Result<Self, Error> {
        let bind_addr = to_addr(bind_addr)?;

        let listener = tokio::net::TcpListener::bind(bind_addr)
            .await
            .map_err(|err| Error::RegistryCreateError {
                err: err.to_string(),
            })?;
        let registrants = Arc::new(Mutex::new(HashMap::new()));
        let (to_gadget, from_registry) = tokio::sync::mpsc::unbounded_channel();
        Ok(ZkNetworkService::King {
            listener: Arc::new(Mutex::new(Some(listener))),
            to_outbound_txs: Arc::new(RwLock::new(HashMap::new())),
            zk_setup: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            registry_id,
            registrants,
            to_gadget,
            identity,
            inbound_messages: Arc::new(Mutex::new(from_registry)),
        })
    }

    pub async fn new_client<T: std::net::ToSocketAddrs>(
        king_registry_addr: T,
        registrant_id: RegistantId,
        client_identity: RustlsCertificate,
        king_certs: RootCertStore,
    ) -> Result<Self, Error> {
        let king_registry_addr = to_addr(king_registry_addr)?;
        let cert_der = client_identity.cert.0.clone();

        let connection = TcpStream::connect(king_registry_addr)
            .await
            .map_err(|err| Error::RegistryCreateError {
                err: err.to_string(),
            })?;

        log::info!(
            "Party {registrant_id} connected to king registry at {}",
            king_registry_addr
        );

        // Upgrade to TLS
        let tls = mpc_net::prod::create_client_mutual_tls_connector(king_certs, client_identity)
            .map_err(|err| Error::RegistryCreateError {
                err: format!("{err:?}"),
            })?;

        let connection = tls
            .connect(
                rustls::ServerName::IpAddress(king_registry_addr.ip()),
                connection,
            )
            .await
            .map_err(|err| Error::RegistryCreateError {
                err: err.to_string(),
            })?;

        let (to_gadget, from_registry) = tokio::sync::mpsc::unbounded_channel();
        let (local_to_outbound_tx, local_to_outbound_rx) = tokio::sync::mpsc::unbounded_channel();

        let connection = TlsStream::Client(connection);

        handle_single_connection(connection, local_to_outbound_rx, to_gadget);

        let mut this = ZkNetworkService::Client {
            king_registry_addr,
            local_to_outbound_tx,
            king_registry_id: None,
            registry_id: registrant_id,
            cert_der,
            inbound_messages: Arc::new(Mutex::new(from_registry)),
            registry_map: Arc::new(Default::default()),
        };

        this.client_register().await?;

        Ok(this)
    }

    async fn client_register(&mut self) -> Result<(), Error> {
        match self {
            Self::King { .. } => Err(Error::RegistryCreateError {
                err: "Cannot register as king".to_string(),
            }),
            Self::Client {
                king_registry_addr: _,
                registry_id: registrant_id,
                local_to_outbound_tx,
                inbound_messages,
                cert_der,
                king_registry_id,
                ..
            } => {
                local_to_outbound_tx
                    .send(RegistryPacket::Register {
                        id: *registrant_id,
                        cert_der: cert_der.clone(),
                    })
                    .map_err(|err| Error::RegistrySendError {
                        err: err.to_string(),
                    })?;

                let response = inbound_messages.lock().await.recv().await.ok_or(
                    Error::RegistryCreateError {
                        err: "No response received".to_string(),
                    },
                )?;

                match response {
                    RegistryPacket::RegisterResponse {
                        id: _,
                        success,
                        king_registry_id: king_id,
                    } => {
                        if !success {
                            return Err(Error::RegistryCreateError {
                                err: "Registration failed".to_string(),
                            });
                        }

                        *king_registry_id = Some(king_id);
                    }
                    _ => {
                        return Err(Error::RegistryCreateError {
                            err: "Unexpected response".to_string(),
                        });
                    }
                }

                Ok(())
            }
        }
    }

    async fn handle_zk_setup_packet(&self, packet: ZkSetupPacket) {
        match self {
            Self::King { zk_setup, .. } => {
                if let ZkSetupPacket::ClientToKing { job_id, .. } = &packet {
                    let mut lock = zk_setup.write().await;
                    if let Some(handle) = lock.get_mut(job_id) {
                        if let Some(handle) = handle.lock().await.tx.take() {
                            if let Err(err) = handle.send(packet) {
                                log::error!(
                                    "Failed to send ZkSetupPacket to local king listener: {err:?}"
                                );
                            }
                        } else {
                            log::error!("King received ZkSetupPacket for job_id that already submitted a result");
                        }
                    } else {
                        // Insert handle that way a local user can wait for the result
                        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                        let job_id = *job_id;
                        tx.send(packet).expect("Should send");

                        let king_handle = KingRegistryResult {
                            tx: Some(tx),
                            rx: Some(rx),
                        };

                        lock.insert(job_id, Arc::new(Mutex::new(king_handle)));
                    }
                } else {
                    log::error!("King received invalid ZkSetupPacket");
                }
            }

            Self::Client { registry_map, .. } => {
                if let ZkSetupPacket::KingToClient { job_id, party_ids } = packet {
                    let mut lock = registry_map.lock().await;
                    if let Some(handle) = lock.get_mut(&job_id) {
                        if let Some(handle) = handle.tx.take() {
                            if let Err(err) = handle.send(party_ids) {
                                log::error!(
                                    "Failed to send ZkSetupPacket to local king listener: {err:?}"
                                );
                            }
                        } else {
                            log::error!("Client received ZkSetupPacket for job_id that already submitted a result");
                        }
                    } else {
                        // Insert handle that way a local user can wait for the result
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        tx.send(party_ids).expect("Should send");

                        let client_handle = ClientRegistryResult {
                            tx: None,
                            rx: Some(rx),
                        };

                        lock.insert(job_id, client_handle);
                    }
                } else {
                    log::error!("Client received invalid ZkSetupPacket");
                }
            }
        }
    }

    pub fn my_id(&self) -> RegistantId {
        match self {
            Self::King { registry_id, .. } | Self::Client { registry_id, .. } => {
                registry_id.clone()
            }
        }
    }

    pub async fn king_only_next_zk_setup_packet(&self, job_id: &[u8; 32]) -> Option<ZkSetupPacket> {
        match self {
            Self::King { zk_setup, .. } => {
                let mut lock = zk_setup.write().await;
                if let Some(handle) = lock.get(job_id).cloned() {
                    // Drop the lock to not block the hashmap
                    drop(lock);
                    if let Some(mut handle) = handle.lock().await.rx.take() {
                        handle.recv().await
                    } else {
                        None
                    }
                } else {
                    // Insert a handle locally to allow delivery of messages
                    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                    let king_handle = Arc::new(Mutex::new(KingRegistryResult {
                        tx: Some(tx),
                        rx: Some(rx),
                    }));

                    lock.insert(*job_id, king_handle.clone());
                    drop(lock);

                    let mut handle_lock = king_handle.lock().await;
                    handle_lock.rx.as_mut().expect("Should exist").recv().await
                }
            }

            Self::Client { .. } => None,
        }
    }

    pub async fn king_only_clear_zk_setup_map_for(&self, job_id: &[u8; 32]) {
        match self {
            Self::King { zk_setup, .. } => {
                zk_setup.write().await.remove(job_id);
            }

            Self::Client { .. } => {}
        }
    }

    pub async fn client_only_get_zk_setup_result(
        &self,
        job_id: &[u8; 32],
    ) -> Result<HashMap<u32, RegistantId>, crate::Error> {
        match self {
            Self::King { .. } => Err(Error::RegistryRecvError {
                err: "Cannot get zk setup result as king".to_string(),
            }),
            Self::Client { registry_map, .. } => {
                let mut lock = registry_map.lock().await;
                if let Some(handle) = lock.get_mut(job_id) {
                    if let Some(handle) = handle.rx.take() {
                        // Drop to not block the hashmap
                        drop(lock);
                        handle.await.map_err(|err| Error::RegistryRecvError {
                            err: format!("Failed to get zk setup result: {:?}", err),
                        })
                    } else {
                        Err(Error::RegistryRecvError {
                            err: "Already received zk setup result".to_string(),
                        })
                    }
                } else {
                    // Insert handle that way a packet that receives the setup packet can send the result here
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let client_handle = ClientRegistryResult {
                        tx: Some(tx),
                        rx: None,
                    };

                    lock.insert(*job_id, client_handle);
                    drop(lock);

                    rx.await.map_err(|err| Error::RegistryRecvError {
                        err: format!("Failed to get zk setup result: {:?}", err),
                    })
                }
            }
        }
    }

    pub fn is_king(&self) -> bool {
        matches! {self, Self::King { .. }}
    }
}

fn to_addr<T: std::net::ToSocketAddrs>(addr: T) -> Result<SocketAddr, Error> {
    addr.to_socket_addrs()
        .map_err(|err| Error::RegistryCreateError {
            err: err.to_string(),
        })?
        .next()
        .ok_or(Error::RegistryCreateError {
            err: "No address found".to_string(),
        })
}

fn handle_single_connection(
    connection: TlsStream<TcpStream>,
    mut local_to_outbound_rx: UnboundedReceiver<RegistryPacket>,
    inbound_to_local_tx: tokio::sync::mpsc::UnboundedSender<RegistryPacket>,
) {
    let (mut sink, mut stream) = mpc_net::multi::wrap_stream(connection).split();
    // Now, take the sink and spawn a task to listen for messages that need to be sent outbound
    tokio::task::spawn(async move {
        while let Some(outbound_message) = local_to_outbound_rx.recv().await {
            if let Err(err) = send_stream(&mut sink, outbound_message).await {
                log::error!("[Registry] Failed to send message to king: {err:?}");
            }
        }
    });

    // Now, the stream will be used to receive messages from the king
    tokio::task::spawn(async move {
        loop {
            match recv_stream(&mut stream).await {
                Ok(message) => {
                    if let Err(err) = inbound_to_local_tx.send(message) {
                        log::error!("[Registry] Failed to send message to gadget: {err:?}");
                        break;
                    }
                }
                Err(Error::RegistryRecvError { err }) => {
                    log::error!("[Registry] Failed to receive message from king: {err:?}");
                    break;
                }
                Err(err) => {
                    log::error!("[Registry] Failed to receive message from king: {err:?}");
                    break;
                }
            }
        }
    });
}

#[derive(Serialize, Deserialize)]
pub enum RegistryPacket {
    Register {
        id: RegistantId,
        cert_der: Vec<u8>,
    },
    RegisterResponse {
        id: RegistantId,
        success: bool,
        king_registry_id: RegistantId,
    },
    // A message for the substrate gadget
    SubstrateGadgetMessage {
        payload: GadgetProtocolMessage,
    },
    ZkSetup {
        payload: ZkSetupPacket,
    },
}

#[derive(Serialize, Deserialize)]
pub enum ZkSetupPacket {
    ClientToKing {
        job_id: [u8; 32],
        party_id: u32,
        registry_id: RegistantId,
    },
    KingToClient {
        job_id: [u8; 32],
        party_ids: HashMap<u32, RegistantId>,
    },
}

fn handle_stream_as_king(
    tls_acceptor: TlsAcceptor,
    stream: TcpStream,
    peer_addr: SocketAddr,
    registrants: Arc<Mutex<HashMap<RegistantId, Registrant>>>,
    to_outbound_txs: Arc<RwLock<HashMap<RegistantId, UnboundedSender<RegistryPacket>>>>,
    to_gadget: UnboundedSender<RegistryPacket>,
    king_registry_id: RegistantId,
) {
    tokio::task::spawn(async move {
        let stream = match tls_acceptor.accept(stream).await {
            Ok(stream) => stream,
            Err(err) => {
                log::error!("[Registry] Failed to upgrade connection from {peer_addr}: {err:?}");
                return;
            }
        };

        let stream = TlsStream::Server(stream);
        let wrapped_stream = mpc_net::multi::wrap_stream(stream);
        let (mut sink, mut stream) = wrapped_stream.split();
        let (to_outbound_tx, mut to_outbound_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut peer_id = None;

        // Spawn a task allowing the king to send messages to the peer from the gadget
        tokio::task::spawn(async move {
            while let Some(message) = to_outbound_rx.recv().await {
                if let Err(err) = send_stream(&mut sink, message).await {
                    log::error!("[Registry] Failed to send message to peer {peer_addr}: {err:?}");
                    break;
                }
            }

            log::warn!("to_outbound_rx closed");
        });

        while let Some(Ok(message)) = stream.next().await {
            match bincode2::deserialize::<RegistryPacket>(&message) {
                Ok(packet) => match packet {
                    RegistryPacket::Register { id, cert_der } => {
                        log::info!("[Registry] Received registration for id {id}");
                        to_outbound_txs.write().insert(id, to_outbound_tx.clone());
                        peer_id = Some(id);
                        let mut registrants = registrants.lock().await;
                        registrants.insert(id, Registrant { id, cert_der });
                        if let Err(err) = to_outbound_tx.send(RegistryPacket::RegisterResponse {
                            id,
                            success: true,
                            king_registry_id,
                        }) {
                            log::error!("[Registry] Failed to send registration response: {err:?}");
                        }
                    }
                    RegistryPacket::SubstrateGadgetMessage { payload } => {
                        if let Err(err) =
                            to_gadget.send(RegistryPacket::SubstrateGadgetMessage { payload })
                        {
                            log::error!("[Registry] Failed to send message to gadget: {err:?}");
                        }
                    }
                    _ => {
                        log::info!("[Registry] Received invalid packet");
                    }
                },
                Err(err) => {
                    log::info!("[Registry] Received invalid packet: {err}");
                }
            }
        }

        // Deregister peer
        if let Some(id) = peer_id {
            let mut registrants = registrants.lock().await;
            registrants.remove(&id);
        }

        log::warn!("[Registry] Connection closed to peer {peer_addr}")
    });
}

async fn send_stream<R: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut SplitSink<WrappedStream<R>, Bytes>,
    payload: RegistryPacket,
) -> Result<(), Error> {
    let serialized = bincode2::serialize(&payload).map_err(|err| Error::RegistrySendError {
        err: err.to_string(),
    })?;

    stream
        .send(serialized.into())
        .await
        .map_err(|err| Error::RegistrySendError {
            err: err.to_string(),
        })
}

async fn recv_stream<R: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut SplitStream<WrappedStream<R>>,
) -> Result<RegistryPacket, Error> {
    let message = stream
        .next()
        .await
        .ok_or(Error::RegistryRecvError {
            err: "Stream closed".to_string(),
        })?
        .map_err(|err| Error::RegistryRecvError {
            err: err.to_string(),
        })?;

    let deserialized =
        bincode2::deserialize(&message).map_err(|err| Error::RegistrySerializationError {
            err: err.to_string(),
        })?;

    Ok(deserialized)
}

#[async_trait]
impl Network for ZkNetworkService {
    async fn next_message(
        &self,
    ) -> Option<<WebbWorkManager as WorkManagerInterface>::ProtocolMessage> {
        match self {
            Self::King {
                inbound_messages, ..
            }
            | Self::Client {
                inbound_messages, ..
            } => loop {
                match inbound_messages.lock().await.recv().await {
                    Some(RegistryPacket::SubstrateGadgetMessage { payload }) => {
                        return Some(payload)
                    }
                    Some(RegistryPacket::ZkSetup { payload }) => {
                        self.handle_zk_setup_packet(payload).await
                    }
                    Some(_packet) => {
                        log::error!("[Registry] Received invalid packet");
                    }
                    None => {
                        log::error!("[Registry] Inbound messages closed");
                        return None;
                    }
                }
            },
        }
    }

    #[allow(clippy::collapsible_else_if)]
    async fn send_message(
        &self,
        message: <WebbWorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), Error> {
        if message.from_network_id.is_none() {
            return Err(Error::RegistrySendError {
                err: "No from_network_id in message".to_string(),
            });
        }

        if let Some(to) = message.to_network_id {
            match self {
                Self::Client {
                    local_to_outbound_tx,
                    king_registry_id,
                    ..
                } => {
                    if to != king_registry_id.clone().expect("Should exist") {
                        return Err(Error::RegistrySendError {
                            err: "Cannot send message to non-king as client".to_string(),
                        });
                    }

                    local_to_outbound_tx
                        .send(RegistryPacket::SubstrateGadgetMessage { payload: message })
                        .map_err(|err| Error::RegistrySendError {
                            err: err.to_string(),
                        })
                }

                Self::King {
                    to_outbound_txs, ..
                } => to_outbound_txs
                    .read()
                    .get(&to)
                    .ok_or(Error::RegistrySendError {
                        err: "No connection to registrant".to_string(),
                    })?
                    .send(RegistryPacket::SubstrateGadgetMessage { payload: message })
                    .map_err(|err| Error::RegistrySendError {
                        err: err.to_string(),
                    }),
            }
        } else {
            if let Self::King {
                to_outbound_txs, ..
            } = self
            {
                // Send to ALL peers
                for (_, tx) in to_outbound_txs.read().iter() {
                    tx.send(RegistryPacket::SubstrateGadgetMessage {
                        payload: message.clone(),
                    })
                    .map_err(|err| Error::RegistrySendError {
                        err: err.to_string(),
                    })?;
                }

                Ok(())
            } else {
                Err(Error::RegistrySendError {
                    err: "Cannot broadcast message as client".to_string(),
                })
            }
        }
    }

    async fn run(&self) -> Result<(), Error> {
        match self {
            Self::King {
                listener,
                registrants,
                to_gadget,
                identity,
                to_outbound_txs,
                registry_id,
                ..
            } => {
                let listener = listener.lock().await.take().expect("Should exist");
                let tls_acceptor = create_server_tls_acceptor(identity.clone()).map_err(|err| {
                    Error::RegistryCreateError {
                        err: format!("{err:?}"),
                    }
                })?;

                while let Ok((stream, peer_addr)) = listener.accept().await {
                    log::info!("[Registry] Accepted connection from {peer_addr}, upgrading to TLS");
                    handle_stream_as_king(
                        tls_acceptor.clone(),
                        stream,
                        peer_addr,
                        registrants.clone(),
                        to_outbound_txs.clone(),
                        to_gadget.clone(),
                        registry_id.clone(),
                    );
                }

                Err(Error::RegistryCreateError {
                    err: "Listener closed".to_string(),
                })
            }
            Self::Client { .. } => Ok(()),
        }
    }
}
