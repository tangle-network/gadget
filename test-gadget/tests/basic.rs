#[cfg(test)]
mod tests {
    use futures::stream::FuturesUnordered;
    use futures::TryStreamExt;
    use gadget_core::gadget::manager::GadgetManager;
    use gadget_core::job_manager::SendFuture;
    use std::error::Error;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::time::Duration;
    use test_gadget::error::TestError;
    use test_gadget::gadget::TestGadget;
    use test_gadget::message::{TestProtocolMessage, UserID};
    use test_gadget::test_network::InMemoryNetwork;
    use test_gadget::work_manager::TestAsyncProtocolParameters;
    use tracing_subscriber::fmt::SubscriberBuilder;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    pub fn setup_log() {
        let _ = SubscriberBuilder::default()
            .with_env_filter(EnvFilter::from_default_env())
            .finish()
            .try_init();
    }

    /// Test a basic gadget with a basic blockchain and a basic network.
    /// This test will run multiple async protocols, and will wait for them all to finish.
    #[tokio::test]
    async fn test_basic_async_protocol() -> Result<(), Box<dyn Error>> {
        setup_log();
        const N_PEERS: usize = 5;
        const N_BLOCKS_PER_SESSION: u64 = 10;
        const BLOCK_DURATION: Duration = Duration::from_millis(1000);

        let (network, recv_handles) = InMemoryNetwork::new(N_PEERS);
        let (bc_tx, _bc_rx) = tokio::sync::broadcast::channel(1000);
        let run_tests_at = Arc::new(vec![0, 2, 4]);
        let gadget_futures = FuturesUnordered::new();
        let (count_finished_tx, mut count_finished_rx) = tokio::sync::mpsc::unbounded_channel();

        for (party_id, network_handle) in recv_handles.into_iter().enumerate() {
            // Create a TestGadget
            let count_finished_tx = count_finished_tx.clone();
            let network = network.clone();
            let async_protocol_gen = move |params: TestAsyncProtocolParameters| {
                async_proto_generator(
                    params,
                    count_finished_tx.clone(),
                    network.clone(),
                    N_PEERS,
                    party_id as UserID,
                )
            };

            let gadget = TestGadget::new(
                bc_tx.subscribe(),
                network_handle,
                run_tests_at.clone(),
                async_protocol_gen,
            );

            gadget_futures.push(GadgetManager::new(gadget));
        }

        let blockchain_future =
            test_gadget::blockchain::blockchain(BLOCK_DURATION, N_BLOCKS_PER_SESSION, bc_tx);

        let finished_future = async move {
            let mut count_received = 0;
            let expected_count = N_PEERS * run_tests_at.len();
            while count_finished_rx.recv().await.is_some() {
                count_received += 1;
                log::info!("Received {} finished signals", count_received);
                if count_received == expected_count {
                    return Ok::<(), TestError>(());
                }
            }

            Err(TestError {
                reason: "Didn't receive all finished signals".to_string(),
            })
        };

        let gadgets_future = gadget_futures.try_collect::<Vec<_>>();

        // The gadgets will run indefinitely if properly behaved, and the blockchain future will as well.
        // The finished future will end when all gadgets have finished their async protocols properly
        // Thus, select all three futures

        tokio::select! {
            res0 = blockchain_future => {
                res0?;
            },
            res1 = gadgets_future => {
                res1.map_err(|err| TestError { reason: format!("{err:?}") })
                    .map(|_| ())?;
            },
            res2 = finished_future => {
                res2?;
            }
        }

        Ok(())
    }

    fn async_proto_generator(
        mut params: TestAsyncProtocolParameters,
        on_finish: tokio::sync::mpsc::UnboundedSender<()>,
        network: InMemoryNetwork,
        n_peers: usize,
        party_id: UserID,
    ) -> Pin<Box<dyn SendFuture<'static, ()>>> {
        Box::pin(async move {
            params
                .start_rx
                .take()
                .expect("Already started")
                .await
                .expect("Failed to start");
            // Broadcast a message to each peer
            network
                .broadcast(
                    party_id,
                    TestProtocolMessage {
                        payload: vec![],
                        from: party_id,
                        to: None,
                        associated_block_id: params.associated_block_id,
                        associated_session_id: params.associated_session_id,
                        associated_ssid: params.associated_ssid,
                        associated_task_id: params.associated_task_id,
                    },
                )
                .expect("Failed to send broadcast");
            // Wait to receive a message from each peer
            for _ in 0..(n_peers - 1) {
                let _ = params
                    .protocol_message_rx
                    .recv()
                    .await
                    .expect("Failed to receive message");
            }

            params
                .is_done
                .store(true, std::sync::atomic::Ordering::Relaxed);
            on_finish.send(()).expect("Didn't send on_finish signal");
        })
    }
}
