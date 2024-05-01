use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct ProtocolConfig {
    pub(crate) protocols: Vec<ProtocolToml>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolToml {
    pub internal: bool,
    pub role_types: Vec<String>,
    pub repository: Option<HashMap<String, String>>,
    pub bin_hashes: Option<HashMap<String, String>>,
}
