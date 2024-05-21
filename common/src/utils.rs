use futures::Stream;
use gadget_io::tokio::sync::mpsc::UnboundedReceiver;
use serde::{Deserialize, Serialize};
use sp_core::ecdsa;
use sp_io::EcdsaVerifyError;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub const ECDSA_SIGNATURE_LENGTH: usize = 65;

// constrain output types to have the `Deserialize` trait
pub fn deserialize<'a, T>(data: &'a [u8]) -> Result<T, serde_json::Error>
where
    T: Deserialize<'a>,
{
    serde_json::from_slice::<T>(data)
}

// shorthand for the above when `T` isn't needed in the function body
pub fn serialize(object: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(object)
}

/// A Channel Receiver that can be cloned.
///
/// On the second clone, the original channel will stop receiving new messages
/// and the new channel will start receiving any new messages after the clone.
pub struct CloneableUnboundedReceiver<T> {
    rx: Arc<gadget_io::tokio::sync::Mutex<UnboundedReceiver<T>>>,
    is_in_use: Arc<AtomicBool>,
}

impl<T: Clone> Clone for CloneableUnboundedReceiver<T> {
    fn clone(&self) -> Self {
        // on the clone, we switch the is_in_use flag to false
        // and we return a new channel
        self.is_in_use
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Self {
            rx: self.rx.clone(),
            is_in_use: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl<T> From<UnboundedReceiver<T>> for CloneableUnboundedReceiver<T> {
    fn from(rx: UnboundedReceiver<T>) -> Self {
        Self {
            rx: Arc::new(gadget_io::tokio::sync::Mutex::new(rx)),
            is_in_use: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<T> Stream for CloneableUnboundedReceiver<T> {
    type Item = T;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if !self.is_in_use.load(std::sync::atomic::Ordering::SeqCst) {
            return std::task::Poll::Ready(None);
        }
        let mut rx = match self.rx.try_lock() {
            Ok(rx) => rx,
            Err(_) => return std::task::Poll::Pending,
        };
        let rx = &mut *rx;
        gadget_io::tokio::pin!(rx);
        rx.poll_recv(cx)
    }
}

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

        // let pub_key = sp_io::crypto::secp256k1_ecdsa_recover(&sig, &hash)?;

        let msg = hash;
        let rid =
            libsecp256k1::RecoveryId::parse(if sig[64] > 26 { sig[64] - 27 } else { sig[64] })
                .map_err(|_| EcdsaVerifyError::BadV)?;
        let sig = libsecp256k1::Signature::parse_overflowing_slice(&sig[..64])
            .map_err(|_| EcdsaVerifyError::BadRS)?;
        let msg = libsecp256k1::Message::parse(&msg);
        let pubkey =
            libsecp256k1::recover(&msg, &sig, &rid).map_err(|_| EcdsaVerifyError::BadSignature)?;
        let mut pub_key = [0u8; 64];
        pub_key.copy_from_slice(&pubkey.serialize()[1..65]);

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
