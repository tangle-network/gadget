use crate::error::TestError;
use crate::gadget::TestGadget;
use crate::message::UserID;
use crate::test_network::InMemoryNetwork;
use crate::work_manager::AsyncProtocolGenerator;
use futures::stream::FuturesUnordered;
use futures::TryStreamExt;
use gadget_core::gadget::manager::GadgetManager;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

pub mod blockchain;
pub mod error;
pub mod gadget;
pub mod message;
pub mod test_network;
pub mod work_manager;

pub async fn simulate_test<T: AsyncProtocolGenerator<TestBundle> + 'static + Clone>(
    n_peers: usize,
    n_blocks_per_session: u64,
    block_duration: Duration,
    run_tests_at: Vec<u64>,
    async_proto_gen: T,
) -> Result<(), Box<dyn Error>> {
    let (network, recv_handles) = InMemoryNetwork::new(n_peers);
    let (bc_tx, _bc_rx) = gadget_io::tokio::sync::broadcast::channel(5000);
    let gadget_futures = FuturesUnordered::new();
    let run_tests_at = Arc::new(run_tests_at);
    let (count_finished_tx, mut count_finished_rx) = gadget_io::tokio::sync::mpsc::unbounded_channel();

    for (party_id, network_handle) in recv_handles.into_iter().enumerate() {
        // Create a TestGadget
        let count_finished_tx = count_finished_tx.clone();
        let network = network.clone();
        let test_bundle = TestBundle {
            count_finished_tx,
            network,
            n_peers,
            party_id: party_id as UserID,
        };

        let gadget = TestGadget::new(
            bc_tx.subscribe(),
            network_handle,
            run_tests_at.clone(),
            test_bundle,
            async_proto_gen.clone(),
        );

        gadget_futures.push(async move {
            gadget_io::tokio::task::spawn(async move {
                GadgetManager::new(gadget).await.map_err(|err| TestError {
                    reason: format!("{err:?}"),
                })
            })
            .await
            .map_err(|err| TestError {
                reason: format!("{err:?}"),
            })?
        });
    }

    let blockchain_future = blockchain::blockchain(block_duration, n_blocks_per_session, bc_tx);

    let finished_future = async move {
        let mut count_received = 0;
        let expected_count = n_peers * run_tests_at.len();
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

    gadget_io::tokio::select! {
            res0 = blockchain_future => {
                res0?;
            },
            res1 = gadgets_future => {
                res1?;
            },
            res2 = finished_future => {
                res2?;
            }
    }

    Ok(())
}

#[derive(Clone)]
pub struct TestBundle {
    pub count_finished_tx: gadget_io::tokio::sync::mpsc::UnboundedSender<()>,
    pub network: InMemoryNetwork,
    pub n_peers: usize,
    pub party_id: UserID,
}
