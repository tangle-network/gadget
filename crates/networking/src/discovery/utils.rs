use alloy_primitives::{keccak256, Address};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EcdsaVerifyError {
    #[error("Bad V")]
    BadV,
    #[error("Bad RS")]
    BadRS,
    #[error("Bad Signature")]
    BadSignature,
}

/// Recovers the public key from a secp256k1 ECDSA signature and message hash
///
/// # Arguments
///
/// * `sig` - The 65-byte signature (r, s, v)
/// * `msg` - The 32-byte message hash that was signed
///
/// # Returns
///
/// The uncompressed 64-byte public key (without the 0x04 prefix) on success
///
/// # Errors
///
/// Returns `EcdsaVerifyError` if:
/// * The recovery ID (v) is invalid
/// * The r,s values are invalid
/// * The signature is invalid or malformed
pub fn secp256k1_ecdsa_recover(
    sig: &[u8; 65],
    msg: &[u8; 32],
) -> Result<[u8; 64], EcdsaVerifyError> {
    let rid = libsecp256k1::RecoveryId::parse(if sig[64] > 26 { sig[64] - 27 } else { sig[64] })
        .map_err(|_| EcdsaVerifyError::BadV)?;
    let sig = libsecp256k1::Signature::parse_overflowing_slice(&sig[..64])
        .map_err(|_| EcdsaVerifyError::BadRS)?;
    let msg = libsecp256k1::Message::parse(msg);
    let pubkey =
        libsecp256k1::recover(&msg, &sig, &rid).map_err(|_| EcdsaVerifyError::BadSignature)?;
    let mut res = [0u8; 64];
    res.copy_from_slice(&pubkey.serialize()[1..65]);
    Ok(res)
}

/// Derives an Ethereum address from an uncompressed public key.
///
/// Takes a 64-byte uncompressed public key (without the 0x04 prefix) and returns the corresponding
/// 20-byte Ethereum address by:
/// 1. Computing the Keccak-256 hash of the public key
/// 2. Taking the last 20 bytes of the hash
///
/// # Arguments
///
/// * `pubkey` - The 64-byte uncompressed public key bytes
///
/// # Returns
///
/// The 20-byte Ethereum address derived from the public key
#[must_use]
pub fn get_address_from_pubkey(pubkey: &[u8; 64]) -> Address {
    let hash = keccak256(pubkey);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..32]);
    Address::from_slice(&address)
}

/// Derives an Ethereum address from a compressed public key.
///
/// Takes a compressed public key (33 bytes with 0x02/0x03 prefix), decompresses it to the full
/// uncompressed form, and derives the corresponding 20-byte Ethereum address by:
/// 1. Decompressing the public key to 65 bytes
/// 2. Computing the Keccak-256 hash of the uncompressed key (minus the 0x04 prefix)
/// 3. Taking the last 20 bytes of the hash
///
/// # Arguments
///
/// * `pubkey` - The compressed public key bytes
///
/// # Returns
///
/// The 20-byte Ethereum address derived from the public key
///
/// # Panics
///
/// Panics if the compressed public key is invalid and cannot be parsed
#[must_use]
pub fn get_address_from_compressed_pubkey(pubkey: &[u8]) -> Address {
    let mut pk = [0u8; 33];
    pk[..pubkey.len()].copy_from_slice(pubkey);
    let pk = libsecp256k1::PublicKey::parse_compressed(&pk).unwrap();
    let decompressed: [u8; 65] = pk.serialize();
    let hash = keccak256(&decompressed[1..65]);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..32]);
    Address::from_slice(&address)
}
