use crate::error::TestError;
use crate::gadget::TestFinalityNotification;
use std::time::Duration;

/// Mocks the blockchain timing mechanism
pub async fn blockchain(
    block_duration: Duration,
    blocks_per_session: u64,
    broadcaster: tokio::sync::broadcast::Sender<TestFinalityNotification>,
) -> Result<(), TestError> {
    let mut current_block = 0;
    let mut current_session = 0;

    loop {
        broadcaster
            .send(TestFinalityNotification {
                number: current_block,
                session_id: current_session,
            })
            .map_err(|_| TestError {
                reason: "Failed to broadcast".to_string(),
            })?;

        current_block += 1;
        current_session = current_block / blocks_per_session;

        tokio::time::sleep(block_duration).await;
    }
}
