use sp_core::ecdsa;

pub const ECDSA_SIGNATURE_LENGTH: usize = 65;

/// Recovers the ECDSA public key from a given message and signature.
///
/// # Arguments
///
/// * `data` - The message for which the signature is being verified.
/// * `signature` - The ECDSA signature to be verified.
///
/// # Returns
///
/// Returns a `Result` containing the recovered ECDSA public key as a `Vec<u8>` or an
/// `EcdsaVerifyError` if verification fails.
pub fn recover_ecdsa_pub_key(
    data: &[u8],
    signature: &[u8],
) -> Result<Vec<u8>, sp_io::EcdsaVerifyError> {
    if signature.len() == ECDSA_SIGNATURE_LENGTH {
        let mut sig = [0u8; ECDSA_SIGNATURE_LENGTH];
        sig[..ECDSA_SIGNATURE_LENGTH].copy_from_slice(signature);

        let hash = sp_core::keccak_256(data);

        let pub_key = sp_io::crypto::secp256k1_ecdsa_recover(&sig, &hash)?;
        return Ok(pub_key.to_vec());
    }
    Err(sp_io::EcdsaVerifyError::BadSignature)
}

/// Verifies the signer of a given message using a set of ECDSA public keys.
///
/// Given a vector of ECDSA public keys (`maybe_signers`), a message (`msg`), and an ECDSA
/// signature (`signature`), this function checks if any of the public keys in the set can be a
/// valid signer for the provided message and signature.
///
/// # Arguments
///
/// * `maybe_signers` - A vector of ECDSA public keys that may represent the potential signers.
/// * `msg` - The message for which the signature is being verified.
/// * `signature` - The ECDSA signature to be verified.
///
/// # Returns
///
/// Returns a tuple containing:
/// * An optional ECDSA public key (`Option<ecdsa::Public>`) representing the verified signer. It is
///   `None` if no valid signer is found.
/// * A boolean value (`bool`) indicating whether the verification was successful (`true`) or not
///   (`false`).
pub fn verify_signer_from_set_ecdsa(
    maybe_signers: Vec<ecdsa::Public>,
    msg: &[u8],
    signature: &[u8],
) -> (Option<ecdsa::Public>, bool) {
    let mut signer = None;
    let recovered_result = recover_ecdsa_pub_key(msg, signature);
    let res = if let Ok(data) = recovered_result {
        let recovered = &data[..32];
        maybe_signers.iter().any(|x| {
            if x.0[1..].to_vec() == recovered.to_vec() {
                signer = Some(*x);
                true
            } else {
                false
            }
        })
    } else {
        false
    };

    (signer, res)
}

/// Utility function to create slice of fixed size
pub fn to_slice_33(val: &[u8]) -> Option<[u8; 33]> {
    if val.len() == 33 {
        let mut key = [0u8; 33];
        key[..33].copy_from_slice(val);

        return Some(key);
    }
    None
}
