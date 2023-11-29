// Constants for dkg-gadget

// ================= Common ======================== //
pub const DKG_KEYGEN_PROTOCOL_NAME: &str = "/webb-tools/dkg/keygen/1";

pub const DKG_SIGNING_PROTOCOL_NAME: &str = "/webb-tools/dkg/signing/1";

// ================= Worker ========================== //
pub mod worker {
    pub const ENGINE_ID: sp_runtime::ConsensusEngineId = *b"WDKG";

    pub const STORAGE_SET_RETRY_NUM: usize = 5;

    pub const MAX_SUBMISSION_DELAY: u32 = 3;

    pub const MAX_KEYGEN_RETRIES: usize = 5;

    /// How many blocks to keep the proposal hash in out local cache.
    pub const PROPOSAL_HASH_LIFETIME: u32 = 10;
}

// ============= Signing Manager ======================= //

pub mod signing_manager {
    // the maximum number of tasks that the work manager tries to assign
    pub const MAX_RUNNING_TASKS: usize = 1;

    // the maximum number of tasks that can be enqueued,
    // enqueued here implies not actively running but listening for messages
    pub const MAX_ENQUEUED_TASKS: usize = 20;

    // How often to poll the jobs to check completion status
    pub const JOB_POLL_INTERVAL_IN_MILLISECONDS: u64 = 500;

    // Max potential number of signing sets to generate for every proposal (equal to the number of
    // retries)
    pub const MAX_POTENTIAL_RETRIES_PER_UNSIGNED_PROPOSAL: u16 = u16::MAX - 1;
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

// ============= Keygen Manager ======================= //

pub mod keygen_manager {
    /// only 1 task at a time may run for keygen
    pub const MAX_RUNNING_TASKS: usize = 1;
    /// There should never be any job enqueueing for keygen
    pub const MAX_ENQUEUED_TASKS: usize = 0;
}
