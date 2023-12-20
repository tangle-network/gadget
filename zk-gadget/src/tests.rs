use std::time::Duration;

use crate::mock::*;

#[tokio::test(flavor = "current_thread")]
async fn gadget_starts() {
    new_test_ext().execute_with(|| {
        assert_eq!(1, 1);
    });
    tokio::time::sleep(Duration::from_millis(2000)).await;
}
