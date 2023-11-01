use futures_util::sink::SinkExt;
use futures_util::StreamExt;
use mpc_net::multi::WrappedStream;
use mpc_net::prod::{CertToDer, RustlsCertificate};
use mpc_net::MpcNetError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use tokio_rustls::rustls::server::NoClientAuth;
use tokio_rustls::rustls::{RootCertStore, ServerConfig};
use tokio_rustls::{rustls, TlsAcceptor, TlsStream};

/// Type should correspond to the on-chain identifier of the registrant
pub type RegistantId = u64;

pub enum ZkNetworkService {
    King {
        listener: Option<tokio::net::TcpListener>,
        registrants: Arc<Mutex<HashMap<RegistantId, Registrant>>>,
        to_gadget: tokio::sync::mpsc::UnboundedSender<GadgetProtocolMessage>,
        from_registry: Option<tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>>,
        identity: RustlsCertificate,
    },
    Client {
        king_registry_addr: SocketAddr,
        registrant_id: RegistantId,
        connection: Option<TlsStream<TcpStream>>,
        cert_der: Vec<u8>,
        to_gadget: tokio::sync::mpsc::UnboundedSender<GadgetProtocolMessage>,
        from_registry: Option<tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>>,
    },
}

#[allow(dead_code)]
pub struct Registrant {
    id: RegistantId,
    cert_der: Vec<u8>,
}

use crate::Error;
use webb_gadget::gadget::message::GadgetProtocolMessage;
use webb_gadget::gadget::network::Network;

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
        bind_addr: T,
        identity: RustlsCertificate,
    ) -> Result<Self, Error> {
        let bind_addr: SocketAddr = bind_addr
            .to_socket_addrs()
            .map_err(|err| Error::RegistryCreateError {
                err: err.to_string(),
            })?
            .next()
            .ok_or(Error::RegistryCreateError {
                err: "No address found".to_string(),
            })?;

        let listener = tokio::net::TcpListener::bind(bind_addr)
            .await
            .map_err(|err| Error::RegistryCreateError {
                err: err.to_string(),
            })?;
        let registrants = Arc::new(Mutex::new(HashMap::new()));
        let (to_gadget, from_registry) = tokio::sync::mpsc::unbounded_channel();
        Ok(ZkNetworkService::King {
            listener: Some(listener),
            registrants,
            to_gadget,
            identity,
            from_registry: Some(from_registry),
        })
    }

    pub async fn new_client<T: std::net::ToSocketAddrs>(
        king_registry_addr: T,
        registrant_id: RegistantId,
        client_identity: RustlsCertificate,
        king_certs: RootCertStore,
    ) -> Result<Self, Error> {
        let king_registry_addr: SocketAddr = king_registry_addr
            .to_socket_addrs()
            .map_err(|err| Error::RegistryCreateError {
                err: err.to_string(),
            })?
            .next()
            .ok_or(Error::RegistryCreateError {
                err: "No address found".to_string(),
            })?;

        let cert_der = client_identity.cert.0.clone();

        let connection = TcpStream::connect(king_registry_addr)
            .await
            .map_err(|err| Error::RegistryCreateError {
                err: err.to_string(),
            })?;

        // Upgrade to TLS
        let tls = mpc_net::prod::create_client_mutual_tls_connector(king_certs, client_identity)
            .map_err(|err| Error::RegistryCreateError {
                err: format!("{err:?}"),
            })?;

        let connection = tls
            .connect(
                tokio_rustls::rustls::ServerName::IpAddress(king_registry_addr.ip()),
                connection,
            )
            .await
            .map_err(|err| Error::RegistryCreateError {
                err: err.to_string(),
            })?;

        let (to_gadget, from_registry) = tokio::sync::mpsc::unbounded_channel();

        let mut this = ZkNetworkService::Client {
            king_registry_addr,
            registrant_id,
            cert_der,
            connection: Some(TlsStream::Client(connection)),
            to_gadget,
            from_registry: Some(from_registry),
        };

        this.client_register().await?;

        Ok(this)
    }

    pub async fn run(self) -> Result<(), Error> {
        match self {
            Self::King {
                listener,
                registrants,
                to_gadget,
                identity,
                ..
            } => {
                let listener = listener.expect("Should exist");
                let tls_acceptor = create_server_tls_acceptor(identity).map_err(|err| {
                    Error::RegistryCreateError {
                        err: format!("{err:?}"),
                    }
                })?;

                while let Ok((stream, peer_addr)) = listener.accept().await {
                    println!("[Registry] Accepted connection from {peer_addr}, upgrading to TLS");
                    let stream = tls_acceptor.accept(stream).await.map_err(|err| {
                        Error::RegistryCreateError {
                            err: format!("{err:?}"),
                        }
                    })?;

                    handle_stream_as_king(
                        TlsStream::Server(stream),
                        peer_addr,
                        registrants.clone(),
                        to_gadget.clone(),
                    );
                }

                Err(Error::RegistryCreateError {
                    err: "Listener closed".to_string(),
                })
            }
            Self::Client {
                connection,
                to_gadget,
                ..
            } => {
                let stream = connection.expect("Should exist");
                let mut wrapped_stream = mpc_net::multi::wrap_stream(stream);
                while let Some(Ok(message)) = wrapped_stream.next().await {
                    match bincode2::deserialize::<RegistryPacket>(&message) {
                        Ok(packet) => match packet {
                            RegistryPacket::SubstrateGadgetMessage { payload } => {
                                if let Err(err) = to_gadget.send(payload) {
                                    eprintln!(
                                        "[Registry] Failed to send message to gadget: {err:?}"
                                    );
                                }
                            }
                            _ => {
                                println!("[Registry] Received invalid packet");
                            }
                        },
                        Err(err) => {
                            println!("[Registry] Received invalid packet: {err}");
                        }
                    }
                }

                Err(Error::RegistryListenError {
                    err: "Connection closed".to_string(),
                })
            }
        }
    }

    async fn client_register(&mut self) -> Result<(), Error> {
        match self {
            Self::King { .. } => Err(Error::RegistryCreateError {
                err: "Cannot register as king".to_string(),
            }),
            Self::Client {
                king_registry_addr: _,
                registrant_id,
                connection,
                cert_der,
                ..
            } => {
                let conn = connection.as_mut().expect("Should exist");
                let mut wrapped_stream = mpc_net::multi::wrap_stream(conn);

                send_stream(
                    &mut wrapped_stream,
                    RegistryPacket::Register {
                        id: *registrant_id,
                        cert_der: cert_der.clone(),
                    },
                )
                .await?;

                let response = recv_stream::<RegistryPacket, _>(&mut wrapped_stream).await?;

                if !matches!(
                    &response,
                    &RegistryPacket::RegisterResponse { success: true, .. }
                ) {
                    return Err(Error::RegistryCreateError {
                        err: "Unexpected response".to_string(),
                    });
                }

                Ok(())
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
enum RegistryPacket {
    Register { id: RegistantId, cert_der: Vec<u8> },
    RegisterResponse { id: RegistantId, success: bool },
    // A message for the substrate gadget
    SubstrateGadgetMessage { payload: GadgetProtocolMessage },
}

fn handle_stream_as_king(
    stream: TlsStream<TcpStream>,
    peer_addr: SocketAddr,
    registrants: Arc<Mutex<HashMap<RegistantId, Registrant>>>,
    to_gadget: tokio::sync::mpsc::UnboundedSender<GadgetProtocolMessage>,
) {
    tokio::task::spawn(async move {
        let mut wrapped_stream = mpc_net::multi::wrap_stream(stream);
        let mut peer_id = None;
        while let Some(Ok(message)) = wrapped_stream.next().await {
            match bincode2::deserialize::<RegistryPacket>(&message) {
                Ok(packet) => match packet {
                    RegistryPacket::Register { id, cert_der } => {
                        println!("[Registry] Received registration for id {id}");
                        peer_id = Some(id);
                        let mut registrants = registrants.lock().await;
                        registrants.insert(id, Registrant { id, cert_der });
                        if let Err(err) = send_stream(
                            &mut wrapped_stream,
                            RegistryPacket::RegisterResponse { id, success: true },
                        )
                        .await
                        {
                            eprintln!("[Registry] Failed to send registration response: {err:?}");
                        }
                    }
                    RegistryPacket::SubstrateGadgetMessage { payload } => {
                        if let Err(err) = to_gadget.send(payload) {
                            eprintln!("[Registry] Failed to send message to gadget: {err:?}");
                        }
                    }
                    _ => {
                        println!("[Registry] Received invalid packet");
                    }
                },
                Err(err) => {
                    println!("[Registry] Received invalid packet: {err}");
                }
            }
        }

        // Deregister peer
        if let Some(id) = peer_id {
            let mut registrants = registrants.lock().await;
            registrants.remove(&id);
        }

        eprintln!("[Registry] Connection closed to peer {peer_addr}")
    });
}

async fn send_stream<T: Serialize, R: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut WrappedStream<R>,
    payload: T,
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

async fn recv_stream<T: DeserializeOwned, R: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut WrappedStream<R>,
) -> Result<T, Error> {
    let message = stream
        .next()
        .await
        .ok_or(Error::RegistryRecvError {
            err: "Stream closed".to_string(),
        })?
        .map_err(|err| Error::RegistryRecvError {
            err: err.to_string(),
        })?;

    let deserialized = bincode2::deserialize(&message).map_err(|err| Error::RegistryRecvError {
        err: err.to_string(),
    })?;

    Ok(deserialized)
}

impl Network for ZkNetworkService {
    fn take_message_receiver(&mut self) -> Option<UnboundedReceiver<GadgetProtocolMessage>> {
        match self {
            Self::King { from_registry, .. } => from_registry.take(),
            Self::Client { from_registry, .. } => from_registry.take(),
        }
    }
}
