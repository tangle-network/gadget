#[cfg(test)]
mod tests {
    use crate::tests::client::BlockchainClient;
    use futures_util::stream::FuturesUnordered;
    use futures_util::TryStreamExt;
    use gadget_core::job_manager::SendFuture;
    use mpc_net::{MpcNet, MultiplexedStreamID};
    use std::collections::HashSet;
    use std::error::Error;
    use std::net::SocketAddr;
    use std::pin::Pin;
    use std::time::Duration;
    use tokio::sync::mpsc::UnboundedSender;
    use tracing_subscriber::fmt::SubscriberBuilder;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;
    use webb_gadget::gadget::message::{GadgetProtocolMessage, UserID};
    use webb_gadget::gadget::network::Network;
    use zk_gadget::module::proto_gen::ZkAsyncProtocolParameters;
    use zk_gadget::network::ZkNetworkService;

    pub fn setup_log() {
        let _ = SubscriberBuilder::default()
            .with_env_filter(EnvFilter::from_default_env())
            .finish()
            .try_init();
    }

    pub mod server {
        use parking_lot::Mutex;
        use std::collections::HashMap;
        use std::sync::Arc;
        use tokio::sync::RwLock;
        use tonic::{Request, Response, Status};
        use zk_gadget::client_ext::job_types::ZkJob;

        tonic::include_proto!("test_blockchain_zk");

        #[derive(Default, Clone)]
        pub struct TestBlockchainServer {
            pub latest_header: Arc<Mutex<u64>>,
            pub jobs: Arc<RwLock<HashMap<u64, ZkJob>>>,
        }

        #[tonic::async_trait]
        impl auth_server::Auth for TestBlockchainServer {
            async fn get_latest_header(
                &self,
                _request: Request<GetLatestHeaderRequest>,
            ) -> Result<Response<GetLatestHeaderResponse>, Status> {
                let latest_block_number = *self.latest_header.lock();
                let session_id = 0; // For now
                Ok(Response::new(GetLatestHeaderResponse {
                    latest_block_number,
                    session_id,
                }))
            }

            async fn get_job_circuit_properties(
                &self,
                request: Request<GetJobCircuitPropertiesRequest>,
            ) -> Result<Response<GetJobCircuitPropertiesResponse>, Status> {
                let lock = self.jobs.read().await;
                let job = lock
                    .get(&request.into_inner().job_id)
                    .ok_or(Status::not_found("Job not found"))?;
                Ok(Response::new(GetJobCircuitPropertiesResponse {
                    job_id: job.circuit.job_id,
                    pk: job.circuit.pk.clone(),
                    wasm_uri: job.circuit.wasm_uri.clone(),
                    r1cs_uri: job.circuit.r1cs_uri.clone(),
                }))
            }

            async fn get_job_properties(
                &self,
                request: Request<GetJobPropertiesRequest>,
            ) -> Result<Response<GetJobPropertiesResponse>, Status> {
                let lock = self.jobs.read().await;
                let job = lock
                    .get(&request.into_inner().job_id)
                    .ok_or(Status::not_found("Job not found"))?;
                Ok(Response::new(GetJobPropertiesResponse {
                    job_id: job.properties.job_id,
                    circuit_id: job.properties.circuit_id,
                    public_inputs: job.properties.public_inputs.clone(),
                    pss: job.properties.pss,
                    a_shares: job.properties.a_shares.clone(),
                    ax_shares: job.properties.ax_shares.clone(),
                    qap_shares: job.properties.qap_shares.clone(),
                }))
            }
        }
    }

    pub mod client {
        use async_trait::async_trait;
        use gadget_core::gadget::substrate::Client;
        use sc_client_api::FinalizeSummary;
        use serde::Serialize;
        use sp_runtime::app_crypto::sp_core::Encode;
        use sp_runtime::codec::Decode;
        use std::error::Error;
        use std::net::SocketAddr;
        use std::sync::Arc;
        use std::time::Duration;
        use tokio::sync::Mutex;
        use uuid::Uuid;
        use webb_gadget::{BlockImportNotification, FinalityNotification};
        use zk_gadget::client_ext::job_types::{JobCircuitProperties, JobProperties};
        use zk_gadget::client_ext::ClientWithApi;

        #[derive(Clone)]
        pub struct BlockchainClient {
            client: Arc<Mutex<super::server::auth_client::AuthClient<tonic::transport::Channel>>>,
            latest_received_header: Arc<Mutex<u64>>,
        }

        impl BlockchainClient {
            pub async fn new(server_addr: SocketAddr) -> Result<Self, Box<dyn Error>> {
                let addr = format!("http://{server_addr}");
                let client = super::server::auth_client::AuthClient::connect(addr).await?;
                Ok(Self {
                    client: Arc::new(Mutex::new(client)),
                    latest_received_header: Arc::new(Mutex::new(0)),
                })
            }
        }

        #[async_trait]
        impl Client<TestBlock> for BlockchainClient {
            async fn get_next_finality_notification(
                &self,
            ) -> Option<FinalityNotification<TestBlock>> {
                loop {
                    let response = self.get_latest_finality_notification().await?;
                    let mut lock = self.latest_received_header.lock().await;

                    if response.header.number > *lock {
                        *lock = response.header.number;
                        return Some(response);
                    }

                    // Wait some time before trying again
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }

            async fn get_latest_finality_notification(
                &self,
            ) -> Option<FinalityNotification<TestBlock>> {
                let request = tonic::Request::new(super::server::GetLatestHeaderRequest {});
                let response = self
                    .client
                    .lock()
                    .await
                    .get_latest_header(request)
                    .await
                    .ok()?
                    .into_inner();

                Some(block_number_to_finality_notification(
                    response.latest_block_number,
                ))
            }

            async fn get_next_block_import_notification(
                &self,
            ) -> Option<BlockImportNotification<TestBlock>> {
                futures_util::future::pending().await
            }
        }

        #[async_trait]
        impl ClientWithApi<TestBlock> for BlockchainClient {
            async fn get_job_circuit_properties(
                &self,
                job_id: u64,
            ) -> Result<JobCircuitProperties, webb_gadget::Error> {
                let response = self
                    .client
                    .lock()
                    .await
                    .get_job_circuit_properties(super::server::GetJobCircuitPropertiesRequest {
                        job_id,
                    })
                    .await
                    .map_err(|err| webb_gadget::Error::ClientError {
                        err: err.to_string(),
                    })?
                    .into_inner();

                Ok(JobCircuitProperties {
                    job_id,
                    pk: response.pk,
                    wasm_uri: response.wasm_uri,
                    r1cs_uri: response.r1cs_uri,
                })
            }

            async fn get_job_properties(
                &self,
                job_id: u64,
            ) -> Result<JobProperties, webb_gadget::Error> {
                let response = self
                    .client
                    .lock()
                    .await
                    .get_job_properties(super::server::GetJobPropertiesRequest { job_id })
                    .await
                    .map_err(|err| webb_gadget::Error::ClientError {
                        err: err.to_string(),
                    })?
                    .into_inner();

                Ok(JobProperties {
                    job_id,
                    circuit_id: response.circuit_id,
                    public_inputs: response.public_inputs,
                    pss: response.pss,
                    a_shares: response.a_shares,
                    ax_shares: response.ax_shares,
                    qap_shares: response.qap_shares,
                })
            }
        }

        pub type TestBlock = sp_runtime::testing::Block<XtDummy>;
        #[derive(Encode, Decode, Serialize, Clone, Eq, PartialEq, Debug)]
        pub struct XtDummy;

        impl sp_runtime::traits::Extrinsic for XtDummy {
            type Call = ();
            type SignaturePayload = ();
        }

        fn block_number_to_finality_notification(
            block_number: u64,
        ) -> FinalityNotification<TestBlock> {
            let header = sp_runtime::generic::Header::<u64, _>::new_from_number(block_number);
            let mut slice = [0u8; 32];
            slice[..8].copy_from_slice(&block_number.to_be_bytes());
            // add random uuid to ensure uniqueness
            slice[8..24].copy_from_slice(&Uuid::new_v4().to_u128_le().to_be_bytes());

            let hash = sp_runtime::testing::H256::from(slice);
            let summary = FinalizeSummary {
                header,
                finalized: vec![hash],
                stale_heads: vec![],
            };

            let (tx, _rx) = sc_utils::mpsc::tracing_unbounded("mpsc_finality_notification", 999999);
            FinalityNotification::<TestBlock>::from_summary(summary, tx)
        }
    }

    pub async fn start_blockchain(
        addr: SocketAddr,
        block_duration: Duration,
    ) -> Result<(), String> {
        let zkp_service = server::TestBlockchainServer::default();
        // TODO: insert some jobs into the hashmap inside the zkp service

        let zkp_server = server::auth_server::AuthServer::new(zkp_service.clone())
            .max_decoding_message_size(1024 * 1024 * 1024) // To allow large messages
            .max_encoding_message_size(1024 * 1024 * 1024);

        let ticker = async move {
            loop {
                tokio::time::sleep(block_duration).await;
                let mut lock = zkp_service.latest_header.lock();
                *lock += 1;
            }
        };

        let server = tonic::transport::Server::builder()
            .add_service(zkp_server)
            .serve(addr);

        tokio::select! {
            res0 = server => {
                res0.map_err(|err| err.to_string())?;
            },
            _ = ticker => {
                Err("Ticker stopped")?;
            }
        }

        Ok(())
    }

    fn generate_pub_key_and_priv_key_der<T: Into<String>>(subject_name: T) -> (Vec<u8>, Vec<u8>) {
        let cert =
            rcgen::generate_simple_self_signed([subject_name.into()]).expect("Should compile");
        (
            cert.serialize_der().expect("Should serialize"),
            cert.serialize_private_key_der(),
        )
    }

    const N: usize = 5;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_zk_gadget() -> Result<(), Box<dyn Error>> {
        const BLOCK_DURATION: Duration = Duration::from_millis(6000);

        setup_log();

        let server_addr: SocketAddr = "127.0.0.1:50051".parse()?;
        let king_bind_addr_orig: SocketAddr = "127.0.0.1:50052".parse()?;
        let (king_public_identity_der, king_private_identity_der) =
            generate_pub_key_and_priv_key_der(king_bind_addr_orig.ip().to_string());
        let blockchain_future = tokio::spawn(start_blockchain(server_addr, BLOCK_DURATION));
        tokio::time::sleep(Duration::from_millis(100)).await; // wait for the server to come up

        let (done_tx, mut done_rx) = tokio::sync::mpsc::unbounded_channel();
        let zk_gadgets_futures = FuturesUnordered::new();

        for party_id in 0..N {
            let client = BlockchainClient::new(server_addr).await?;
            let test_config = if party_id == 0 {
                zk_gadget::ZkGadgetConfig {
                    king_bind_addr: Some(king_bind_addr_orig),
                    client_only_king_addr: None,
                    id: party_id as _,
                    public_identity_der: king_public_identity_der.clone(),
                    private_identity_der: king_private_identity_der.clone(),
                    client_only_king_public_identity_der: None,
                }
            } else {
                let (public_identity_der, private_identity_der) =
                    generate_pub_key_and_priv_key_der("localhost");
                zk_gadget::ZkGadgetConfig {
                    king_bind_addr: None,
                    client_only_king_addr: Some(king_bind_addr_orig),
                    id: party_id as _,
                    public_identity_der,
                    private_identity_der,
                    client_only_king_public_identity_der: Some(king_public_identity_der.clone()),
                }
            };

            let additional_parameters = AdditionalParams {
                stop_tx: done_tx.clone(),
                client: client.clone(),
            };

            let zk_gadget_future = zk_gadget::run(
                test_config,
                client,
                additional_parameters,
                async_protocol_generator,
            );
            zk_gadgets_futures.push(Box::pin(zk_gadget_future));
        }

        let done_rx_future = async move {
            let mut done_signals_received = 0;
            while done_signals_received < N {
                done_rx
                    .recv()
                    .await
                    .ok_or_else(|| "Did not receive done signal")?;
                log::info!("Received {}/{} done signals", done_signals_received, N);
                done_signals_received += 1;
            }

            log::info!("Received ALL done signals");

            Ok::<_, String>(())
        };

        let zk_gadget_future = zk_gadgets_futures.try_collect::<Vec<_>>();

        tokio::select! {
            res0 = blockchain_future => {
                res0??;
            },
            res1 = zk_gadget_future => {
                res1?;
            },
            res2 = done_rx_future => {
                res2?;
            }
        }

        Ok(())
    }

    #[derive(Clone)]
    struct AdditionalParams {
        pub stop_tx: UnboundedSender<()>,
        pub client: BlockchainClient,
    }

    fn async_protocol_generator(
        mut params: ZkAsyncProtocolParameters<AdditionalParams, ZkNetworkService>,
    ) -> Pin<Box<dyn SendFuture<'static, Result<(), webb_gadget::Error>>>> {
        Box::pin(async move {
            if params.party_id == 0 {
                // Receive N-1 messages from the other parties
                for party_id in 0..N {
                    let party_id = party_id as u32;
                    if party_id != params.party_id {
                        let message = params
                            .recv_from(party_id, MultiplexedStreamID::Zero)
                            .await
                            .expect("Should receive protocol message");
                    }
                }

                for party_id in 0..N {
                    let party_id = party_id as u32;
                    if party_id != params.party_id {
                        params
                            .send_to(party_id, Default::default(), MultiplexedStreamID::Zero)
                            .await
                            .expect("Should send");
                    }
                }
            } else {
                params
                    .send_to(0, Default::default(), MultiplexedStreamID::Zero)
                    .await
                    .expect("Should send");
                let _message_from_king = params
                    .recv_from(0, MultiplexedStreamID::Zero)
                    .await
                    .expect("Should receive protocol message");
            }

            // TODO: use the params.extra_parameters.client to get job metadata, **AFTER** the server is given some data
            // to store inside its hashmap. By default, there is none. See previous TODO

            params
                .extra_parameters
                .stop_tx
                .send(())
                .expect("Should send");
            Ok(())
        })
    }
}
