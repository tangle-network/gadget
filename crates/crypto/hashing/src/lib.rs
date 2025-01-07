#[cfg(feature = "sha2")]
pub fn sha2_256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;

    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

#[cfg(feature = "sha2")]
pub fn sha2_512(data: &[u8]) -> [u8; 64] {
    use sha2::Digest;

    let mut hasher = sha2::Sha512::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 64];
    hash.copy_from_slice(&result);
    hash
}

#[cfg(feature = "sha3")]
pub fn keccak_256(data: &[u8]) -> [u8; 32] {
    use sha3::Digest;

    let mut hasher = sha3::Keccak256::new();
    hasher.update(data);
    let output = hasher.finalize();
    output.into()
}

#[cfg(feature = "blake3")]
pub fn blake3_256(data: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    let output = hasher.finalize();
    output.into()
}
