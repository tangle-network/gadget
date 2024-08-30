use std::collections::HashMap;

use cggmp21::security_level::SecurityLevel;
use dfns_cggmp21::security_level::SecurityLevel128;
use gadget_common::config::Network;
use generic_ec::Curve;
use sha2::Sha256;
use sp_core::ecdsa;

pub mod keygen;
pub mod refresh;
pub mod sign;

pub type DefaultSecurityLevel = SecurityLevel128;
pub type DefaultCryptoHasher = Sha256;
