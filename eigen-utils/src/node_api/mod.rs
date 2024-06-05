use alloy_transport_http::Http;
use http_body_util::Full;
use hyper::{
    body::{self, Body, Bytes, Incoming}, server::conn::{http1, http2}, service::service_fn, Method, Request, Response, StatusCode
};
use serde::Serialize;
use serde_json::json;
use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{net::TcpListener};
use log::{info, error};

use self::tokiort::TokioIo;

pub mod tokiort;

const BASE_URL: &str = "/eigen";
const SPEC_SEM_VER: &str = "v0.0.1";

#[derive(Debug, Clone, Copy)]
enum NodeHealth {
    Healthy,
    PartiallyHealthy,
    Unhealthy,
}

#[derive(Debug, Serialize, Clone, Copy)]
enum ServiceStatus {
    Up,
    Down,
    Initializing,
}

#[derive(Debug, Serialize, Clone)]
struct NodeService {
    id: String,
    name: String,
    description: String,
    status: ServiceStatus,
}

#[derive(Clone)]
struct NodeApi {
    avs_node_name: String,
    avs_node_sem_ver: String,
    health: Arc<Mutex<NodeHealth>>,
    node_services: Arc<Mutex<Vec<NodeService>>>,
}

impl NodeApi {
    fn new(avs_node_name: &str, avs_node_sem_ver: &str) -> Self {
        Self {
            avs_node_name: avs_node_name.to_string(),
            avs_node_sem_ver: avs_node_sem_ver.to_string(),
            health: Arc::new(Mutex::new(NodeHealth::Healthy)),
            node_services: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn update_health(&self, health: NodeHealth) {
        let mut health_lock = self.health.lock().unwrap();
        *health_lock = health;
    }

    fn register_new_service(&self, service: NodeService) {
        let mut node_services_lock = self.node_services.lock().unwrap();
        node_services_lock.push(service);
    }

    fn update_service_status(&self, service_id: &str, service_status: ServiceStatus) -> Result<(), String> {
        let mut node_services_lock = self.node_services.lock().unwrap();
        for service in node_services_lock.iter_mut() {
            if service.id == service_id {
                service.status = service_status;
                return Ok(());
            }
        }
        Err(format!("Service with serviceId {} not found", service_id))
    }

    fn deregister_service(&self, service_id: &str) -> Result<(), String> {
        let mut node_services_lock = self.node_services.lock().unwrap();
        if let Some(index) = node_services_lock.iter().position(|service| service.id == service_id) {
            node_services_lock.remove(index);
            return Ok(());
        }
        Err(format!("Service with serviceId {} not found", service_id))
    }

    async fn start(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(addr).await.unwrap();
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let io = TokioIo::new(stream);
            let this = Arc::new(self.clone());
            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(move |req| {
                        let this = Arc::clone(&this);
                        async move { this.router(req).await }
                    }))
                    .await
                {
                    println!("Error serving connection: {:?}", err);
                }
            });
        }
    }

    async fn router(&self, req: Request<body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
        match (req.method(), req.uri().path()) {
            (&Method::GET, path) if path == format!("{}/node", BASE_URL) => {
                self.node_handler().await
            }
            (&Method::GET, path) if path == format!("{}/node/health", BASE_URL) => {
                self.health_handler().await
            }
            (&Method::GET, path) if path == format!("{}/node/services", BASE_URL) => {
                self.services_handler().await
            }
            (&Method::GET, path) if path.starts_with(&format!("{}/node/services/", BASE_URL)) => {
                self.service_health_handler(req).await
            }
            _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Not Found")))
                .unwrap()),
        }
    }

    async fn node_handler(&self) -> Result<Response<Full<Bytes>>, Infallible> {
        let response = json!({
            "node_name": self.avs_node_name,
            "spec_version": SPEC_SEM_VER,
            "node_version": self.avs_node_sem_ver,
        });

        Ok(json_response(response))
    }

    async fn health_handler(&self) -> Result<Response<Full<Bytes>>, Infallible> {
        let health_lock = self.health.lock().unwrap();
        match *health_lock {
            NodeHealth::Healthy => Ok(Response::new(Full::new(Bytes::from("")))),
            NodeHealth::PartiallyHealthy => Ok(Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .body(Full::new(Bytes::from("")))
                .unwrap()),
            NodeHealth::Unhealthy => Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Full::new(Bytes::from("")))
                .unwrap()),
        }
    }

    async fn services_handler(&self) -> Result<Response<Full<Bytes>>, Infallible> {
        let node_services_lock = self.node_services.lock().unwrap();
        let response = json!({
            "services": *node_services_lock,
        });

        Ok(json_response(response))
    }

    async fn service_health_handler(&self, req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
        let path = req.uri().path();
        let service_id = path.trim_start_matches(&format!("{}/node/services/", BASE_URL)).trim_end_matches("/health");

        let node_services_lock = self.node_services.lock().unwrap();
        for service in node_services_lock.iter() {
            if service.id == service_id {
                return match service.status {
                    ServiceStatus::Up => Ok(Response::new(Full::new(Bytes::from("")))),
                    ServiceStatus::Down => Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .body(Full::new(Bytes::from("")))
                        .unwrap()),
                    ServiceStatus::Initializing => Ok(Response::builder()
                        .status(StatusCode::PARTIAL_CONTENT)
                        .body(Full::new(Bytes::from("")))
                        .unwrap()),
                };
            }
        }

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("")))
            .unwrap())
    }
}

fn json_response<T: Serialize>(data: T) -> Response<Full<Bytes>> {
    let body = serde_json::to_string(&data).unwrap();
    Response::builder()
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}
