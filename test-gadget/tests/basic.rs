#[cfg(test)]
mod tests {
    use gadget_core::job::JobError;
    use gadget_core::job_manager::SendFuture;
    use std::error::Error;
    use std::pin::Pin;
    use std::time::Duration;
    use test_gadget::message::TestProtocolMessage;
    use test_gadget::work_manager::TestAsyncProtocolParameters;
    use test_gadget::TestBundle;
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
        test_gadget::simulate_test(
            5,
            10,
            Duration::from_millis(1000),
            vec![0, 2, 4],
            async_proto_generator,
        )
        .await
    }

    fn async_proto_generator(
        mut params: TestAsyncProtocolParameters<TestBundle>,
    ) -> Pin<Box<dyn SendFuture<'static, Result<(), JobError>>>> {
        let n_peers = params.test_bundle.n_peers;
        let party_id = params.test_bundle.party_id;
        Box::pin(async move {
            params
                .start_rx
                .take()
                .expect("Already started")
                .await
                .expect("Failed to start");
            // Broadcast a message to each peer
            params
                .test_bundle
                .network
                .broadcast(
                    party_id,
                    TestProtocolMessage {
                        payload: vec![],
                        from: party_id,
                        to: None,
                        associated_block_id: params.associated_block_id,
                        associated_session_id: params.associated_session_id,
                        associated_retry_id: params.associated_retry_id,
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
            params
                .test_bundle
                .count_finished_tx
                .send(())
                .expect("Didn't send on_finish signal");
            Ok::<_, JobError>(())
        })
    }
}
