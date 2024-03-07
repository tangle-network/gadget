use dfns_cggmp21::supported_curves::Secp256k1;
use gadget_common::gadget::message::UserID;
use gadget_core::job::JobError;
use rand::prelude::SliceRandom;
use sp_core::ecdsa;
use sp_core::ecdsa::Public;
use std::collections::HashMap;

pub type ChosenSigners = (u16, Vec<u16>, HashMap<UserID, Public>);

/// Given a list of participants, choose `t` of them and return the index of the current participant
/// and the indices of the chosen participants, as well as a mapping from the index to the account
/// id.
///
/// # Errors
/// If we are not selected to sign the message it will return an error
/// [`gadget_common::Error::ParticipantNotSelected`].
///
/// # Panics
/// If the current participant is not in the list of participants it will panic.
pub fn choose_signers<R: rand::Rng>(
    rng: &mut R,
    my_role_key: &ecdsa::Public,
    participants: &[ecdsa::Public],
    t: u16,
) -> Result<ChosenSigners, gadget_common::Error> {
    let selected_participants = participants
        .choose_multiple(rng, t as usize)
        .cloned()
        .collect::<Vec<_>>();

    let selected_participants_indices = selected_participants
        .iter()
        .map(|p| participants.iter().position(|x| x == p).unwrap() as u16)
        .collect::<Vec<_>>();

    let j = participants
        .iter()
        .position(|p| p == my_role_key)
        .expect("Should exist") as u16;

    let i = selected_participants_indices
        .iter()
        .position(|p| p == &j)
        .map(|i| i as u16)
        .ok_or_else(|| gadget_common::Error::ParticipantNotSelected {
            id: *my_role_key,
            reason: String::from("we are not selected to sign"),
        })?;

    let user_id_to_account_id_mapping = selected_participants
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, p)| (i as UserID, p))
        .collect();
    Ok((
        i,
        selected_participants_indices,
        user_id_to_account_id_mapping,
    ))
}

pub trait SignatureVerifier {
    fn verify_signature(
        signature_bytes: [u8; 65],
        data_hash: &[u8; 32],
        public_key_bytes: &[u8],
    ) -> Result<[u8; 65], JobError>;
}

impl SignatureVerifier for Secp256k1 {
    fn verify_signature(
        mut signature_bytes: [u8; 65],
        data_hash: &[u8; 32],
        public_key_bytes: &[u8],
    ) -> Result<[u8; 65], JobError> {
        if public_key_bytes.is_empty() {
            return Err(JobError {
                reason: "Public key is empty".to_string(),
            });
        }

        // To figure out the recovery ID, we need to try all possible values of v
        // in our case, v can be 0 or 1
        let mut v = 0u8;
        loop {
            let mut signature_bytes = signature_bytes;
            signature_bytes[64] = v;
            let res = sp_io::crypto::secp256k1_ecdsa_recover(&signature_bytes, &data_hash);
            match res {
                Ok(key) if key[..32] == public_key_bytes[1..] => {
                    // Found the correct v
                    signature_bytes[64] = v + 27;
                    return Ok(signature_bytes);
                }
                Ok(_) => {
                    // Found a key, but not the correct one
                    // Try the other v value
                    if v == 1 {
                        // We tried both v values, but no key was found
                        // This should never happen, but if it does, we will just
                        // leave v as 1 and break
                        return Err(JobError {
                            reason: "Failed to recover signature (both v=[0,1] did not match)"
                                .to_string(),
                        });
                    }
                    v = 1;
                    continue;
                }
                Err(_) if v == 1 => {
                    // We tried both v values, but no key was found
                    // This should never happen, but if it does, we will just
                    // leave v as 1 and break
                    return Err(JobError {
                        reason:
                        "Failed to recover signature (both v=[0,1] errored and/or did not match)"
                            .to_string(),
                    });
                }
                Err(_) => {
                    // No key was found, try the other v value
                    v = 1;
                    continue;
                }
            }
        }
    }
}
