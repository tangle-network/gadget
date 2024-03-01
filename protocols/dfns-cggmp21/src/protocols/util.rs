use gadget_common::gadget::message::UserID;
use itertools::Itertools;
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
    _rng: &mut R,
    my_account_id: &Public,
    participants: &[Public],
    t: u16,
) -> Result<ChosenSigners, gadget_common::Error> {
    let selected_participants = participants
        .iter()
        .take(t as usize)
        .copied()
        .enumerate()
        .map(|(i, p)| (i as UserID, p))
        .sorted_by_key(|k| k.0)
        .collect::<HashMap<_, _>>();

    let selected_participants_indices = selected_participants
        .iter()
        .map(|p| participants.iter().position(|x| x == p.1).unwrap() as u16)
        .sorted()
        .collect::<Vec<_>>();

    // Generate a new mapping of part indexes starting from 0 and incrementing to t (e.g., [0, 1, 2, ...])
    let _user_id_to_account_id_mapping = selected_participants_indices
        .iter()
        .map(|idx| participants[*idx as usize])
        .enumerate()
        .map(|(i, p)| (i as UserID, p))
        .collect::<HashMap<UserID, Public>>();

    // Find our position in the NEW mapping
    let my_position = *selected_participants
        .iter()
        .find(|(_id, pk)| pk == &my_account_id)
        .ok_or_else(|| gadget_common::Error::ParticipantNotSelected {
            id: *my_account_id,
            reason: "We are not signing this round".to_string(),
        })?
        .0;

    Ok((
        my_position as u16,
        selected_participants_indices,
        selected_participants,
    ))
}
