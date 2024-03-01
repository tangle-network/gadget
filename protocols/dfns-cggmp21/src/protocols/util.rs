use gadget_common::gadget::message::UserID;
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
