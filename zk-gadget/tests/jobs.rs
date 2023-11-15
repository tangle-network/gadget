#[cfg(test)]
mod tests {
    use futures_util::stream::FuturesUnordered;
    use futures_util::TryStreamExt;
    use std::error::Error;
    use std::net::SocketAddr;
    use std::time::Duration;

    pub mod server {
        use parking_lot::Mutex;
        use std::sync::Arc;
        use tonic::{Request, Response, Status};
        tonic::include_proto!("test_blockchain_zk");

        #[derive(Default, Clone)]
        pub struct TestBlockchainServer {
            pub latest_header: Arc<Mutex<u64>>,
        }

        #[tonic::async_trait]
        impl auth_server::Auth for TestBlockchainServer {
            async fn get_latest_header(
                &self,
                request: Request<GetLatestHeaderRequest>,
            ) -> Result<Response<GetLatestHeaderResponse>, Status> {
                let latest_block_number = *self.latest_header.lock();
                let session_id = 0; // For now
                Ok(Response::new(GetLatestHeaderResponse {
                    latest_block_number,
                    session_id,
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
        use std::time::Duration;
        use tokio::sync::Mutex;
        use uuid::Uuid;
        use webb_gadget::{BlockImportNotification, FinalityNotification};

        pub struct BlockchainClient {
            client: Mutex<super::server::auth_client::AuthClient<tonic::transport::Channel>>,
            latest_received_header: Mutex<u64>,
        }

        impl BlockchainClient {
            pub async fn new(server_addr: SocketAddr) -> Result<Self, Box<dyn Error>> {
                let addr = format!("http://{server_addr}");
                let client = super::server::auth_client::AuthClient::connect(addr).await?;
                Ok(Self {
                    client: Mutex::new(client),
                    latest_received_header: Mutex::new(0),
                })
            }
        }

        #[async_trait]
        impl Client<TestBlock> for BlockchainClient {
            async fn get_next_finality_notification(
                &self,
            ) -> Option<FinalityNotification<TestBlock>> {
                loop {
                    let request = tonic::Request::new(super::server::GetLatestHeaderRequest {});
                    let response = self
                        .client
                        .lock()
                        .await
                        .get_latest_header(request)
                        .await
                        .ok()?
                        .into_inner();
                    let mut lock = self.latest_received_header.lock().await;

                    if response.latest_block_number > *lock {
                        *lock = response.latest_block_number;
                        let finality_notification =
                            block_number_to_finality_notification(response.latest_block_number);
                        return Some(finality_notification);
                    }

                    // Wait some time before trying again
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }

            async fn get_next_block_import_notification(
                &self,
            ) -> Option<BlockImportNotification<TestBlock>> {
                futures_util::future::pending().await
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
            let notification = FinalityNotification::<TestBlock>::from_summary(summary, tx);
        }
    }

    pub async fn start_blockchain(
        addr: SocketAddr,
        block_duration: Duration,
    ) -> Result<(), Box<dyn Error>> {
        let zkp_service = server::TestBlockchainServer::default();

        let zkp_server = server::auth_server::AuthServer::new(zkp_service)
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
                res0?;
            },
            _ = ticker => {
                Err("Ticker stopped")?;
            }
        }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_zk_gadget() -> Result<(), Box<dyn Error>> {
        const BLOCK_DURATION: Duration = Duration::from_millis(10000);
        const N: usize = 5;

        let server_addr: SocketAddr = "127.0.0.1:50051".parse()?;
        let client = client::BlockchainClient::new(server_addr).await?;
        let blockchain_future = tokio::spawn(start_blockchain(server_addr, BLOCK_DURATION));
        tokio::time::sleep(Duration::from_millis(100)).await; // wait for the server to come up

        let zk_gadgets_futures = FuturesUnordered::new();
        for party_id in 0..N {
            let test_config = zk_gadget::ZkGadgetConfig {
                king_bind_addr: None,
                client_only_king_addr: Some(server_addr),
                id: party_id as _,
                public_identity_der: vec![],
                private_identity_der: vec![],
                client_only_king_public_identity_der: None,
            };
            let zk_gadget_future = zk_gadget::run(test_config, client.clone());
            zk_gadgets_futures.push(Box::pin(zk_gadget_future));
        }

        let zk_gadget_future = zk_gadgets_futures.try_collect::<Vec<_>>();

        tokio::select! {
            res0 = blockchain_future => {
                res0?;
            },
            res1 = zk_gadget_future => {
                res1?;
            }
        }

        Ok(())
    }
}
