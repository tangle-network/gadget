use alloy_primitives::{keccak256, Address};
use thiserror::*;

#[derive(Error, Debug)]
pub enum EcdsaVerifyError {
    #[error("Bad V")]
    BadV,
    #[error("Bad RS")]
    BadRS,
    #[error("Bad Signature")]
    BadSignature,
}

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

pub fn get_address_from_pubkey(pubkey: &[u8; 64]) -> Address {
    let hash = keccak256(pubkey);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..32]);
    Address::from_slice(&address)
}

pub fn get_address_from_compressed_pubkey(pubkey: Vec<u8>) -> Address {
    let mut pk = [0u8; 33];
    pk[..pubkey.len()].copy_from_slice(&pubkey);
    let pk = libsecp256k1::PublicKey::parse_compressed(&pk).unwrap();
    let decompressed: [u8; 65] = pk.serialize();
    let hash = keccak256(&decompressed[1..65]);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..32]);
    Address::from_slice(&address)
}
