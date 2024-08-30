use dfns_cggmp21::security_level::SecurityLevel128;
use sha2::Sha256;

pub mod keygen;
pub mod sign;
pub mod refresh;

pub type DefaultSecurityLevel = SecurityLevel128;
pub type DefaultCryptoHasher = Sha256;