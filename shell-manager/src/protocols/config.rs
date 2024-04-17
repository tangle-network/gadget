use std::collections::HashMap;

pub struct ProtocolConfig {
    protocols: Vec<Protocol>
}

pub struct Protocol {
    pub internal: bool,
    pub role_types: Vec<String>,
    pub repository: Option<HashMap<String, String>>,
    pub bin_hashes: Option<HashMap<String, String>>,
}