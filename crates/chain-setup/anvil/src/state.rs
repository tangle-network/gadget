use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnvilState {
    pub accounts: HashMap<String, AccountState>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountState {
    pub nonce: u64,
    pub balance: String,
    pub code: String,
    pub storage: HashMap<String, String>,
}

/// Get the default Anvil state as a raw JSON string
#[must_use]
pub fn get_default_state_json() -> &'static str {
    DEFAULT_STATE
}

/// Get the default Anvil state parsed into the [`AnvilState`] struct
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_default_state() -> AnvilState {
    serde_json::from_str(DEFAULT_STATE).expect("Failed to parse default state JSON")
}

// The default state JSON data - stored like this for simplicity
const DEFAULT_STATE: &str = include_str!("../data/state.json");
