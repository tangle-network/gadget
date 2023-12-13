// Constants for dkg-gadget

// ================= Common ======================== //
pub const DKG_KEYGEN_PROTOCOL_NAME: &str = "/webb-tools/dkg/keygen/1";
pub const DKG_SIGNING_PROTOCOL_NAME: &str = "/webb-tools/dkg/signing/1";

// ============= Signing Worker ======================= //

pub mod signing_worker {
    use std::time::Duration;

    // the maximum number of tasks that the work manager tries to assign
    pub const MAX_RUNNING_TASKS: usize = 1;

    // the maximum number of tasks that can be enqueued,
    // enqueued here implies not actively running but listening for messages
    pub const MAX_ENQUEUED_TASKS: usize = 20;

    // How often to poll the jobs to check completion status
    pub const JOB_POLL_INTERVAL: Duration = Duration::from_millis(500);
}

// ============= Networking ======================= //

pub mod network {
    /// Maximum number of known messages hashes to keep for a peer.
    pub const MAX_KNOWN_MESSAGES: usize = 4096;

    /// Maximum allowed size for a DKG Signed Message notification.
    pub const MAX_MESSAGE_SIZE: u64 = 16 * 1024 * 1024;

    /// Maximum number of duplicate messages that a single peer can send us.
    ///
    /// This is to prevent a malicious peer from spamming us with messages.
    pub const MAX_DUPLICATED_MESSAGES_PER_PEER: usize = 8;
}

// ============= Keygen Worker ======================= //

pub mod keygen_worker {
    /// only 1 task at a time may run for keygen
    pub const MAX_RUNNING_TASKS: usize = 1;
    /// There should never be any job enqueueing for keygen
    pub const MAX_ENQUEUED_TASKS: usize = 0;
}
