// Constants for threshold BLS gadget

// ================= Common ======================== //
pub const THRESHOLD_BLS_KEYGEN_PROTOCOL_NAME: &str = "/tangle/bls/keygen/1";
pub const THRESHOLD_BLS_SIGNING_PROTOCOL_NAME: &str = "/tangle/bls/signing/1";

// ============= Signing Protocol ======================= //

pub mod signing_worker {
    use std::time::Duration;

    // the maximum number of tasks that the work manager tries to assign
    pub const MAX_RUNNING_TASKS: usize = 2;

    // the maximum number of tasks that can be enqueued,
    // enqueued here implies not actively running but listening for messages
    pub const MAX_ENQUEUED_TASKS: usize = 10;

    // How often to poll the jobs to check completion status
    pub const JOB_POLL_INTERVAL: Duration = Duration::from_millis(500);
}

// ============= Keygen Protocol ======================= //

pub mod keygen_worker {
    /// only 1 task at a time may run for keygen
    pub const MAX_RUNNING_TASKS: usize = 2;
    /// There should never be any job enqueueing for keygen
    pub const MAX_ENQUEUED_TASKS: usize = 10;
}
