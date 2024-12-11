use crate::error::Error;
use std::path::PathBuf;

/// Core configuration trait that all configurations must implement
pub trait ConfigCore {
    /// Get the data directory for this configuration
    fn data_dir(&self) -> Option<&PathBuf>;

    /// Get the protocol this configuration is for
    fn protocol(&self) -> Protocol;
}

#[cfg(feature = "keystore")]
pub trait KeystoreConfig: ConfigCore {
    /// Initialize a keystore from this configuration
    fn init_keystore(&self) -> Result<gadget_keystore::keystore::Keystore, Error>;

    /// Get the keystore URI
    fn keystore_uri(&self) -> &str;

    /// Get the keystore data directory
    fn data_dir(&self) -> Option<&PathBuf>;
}

#[cfg(feature = "networking")]
pub trait NetworkConfig: ConfigCore {
    /// Get the target address
    fn target_addr(&self) -> std::net::IpAddr;

    /// Get the target port
    fn target_port(&self) -> u16;

    /// Get the bootnodes
    fn bootnodes(&self) -> &[libp2p::Multiaddr];

    /// Whether to use secure URLs
    fn use_secure_url(&self) -> bool;
}

#[cfg(feature = "tangle")]
pub trait TangleConfig: ConfigCore {
    /// Get the blueprint ID
    fn blueprint_id(&self) -> u64;

    /// Get the service ID if available
    fn service_id(&self) -> Option<u64>;
}

#[cfg(feature = "eigenlayer")]
pub trait EigenlayerConfig: ConfigCore {
    /// Get the Eigenlayer contract addresses
    fn contract_addresses(&self) -> &EigenlayerContractAddresses;
}
