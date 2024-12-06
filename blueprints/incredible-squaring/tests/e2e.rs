use blueprint_test_utils::{test_tangle_blueprint, InputValue, OutputValue};

const SQUARING_JOB_ID: usize = 0;
const N: usize = 5;

test_tangle_blueprint!(
    N,                         // Number of nodes
    SQUARING_JOB_ID,           // Job ID
    [InputValue::Uint64(5)],   // Inputs
    [OutputValue::Uint64(25)]  // Expected output: input squared
);
