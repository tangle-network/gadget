#![allow(clippy::type_complexity, clippy::too_many_arguments)]
//! When delivering messages to an async protocol, we want to make sure we don't mix up voting and public key gossip messages
//! Thus, this file contains a function that takes a channel from the gadget to the async protocol and splits it into two channels
use gadget_common::gadget::message::UserID;
use rand::seq::SliceRandom;
use sp_core::ecdsa::Public;
use std::collections::HashMap;

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
    my_account_id: &Public,
    participants: &[Public],
    t: u16,
) -> Result<(u16, Vec<u16>, HashMap<UserID, Public>), gadget_common::Error> {
    let selected_participants = participants
        .choose_multiple(rng, t as usize)
        .cloned()
        .collect::<Vec<_>>();

    let selected_participants_indices = selected_participants
        .iter()
        .map(|p| participants.iter().position(|x| x == p).unwrap() as u16)
        .collect::<Vec<_>>();

    let mut selected_participants_with_indices: Vec<(u16, Public)> = selected_participants_indices
        .iter()
        .cloned()
        .zip(selected_participants)
        .collect();

    selected_participants_with_indices.sort_by_key(|&(index, _)| index);

    let (sorted_selected_participants_indices, sorted_selected_participants): (
        Vec<u16>,
        Vec<Public>,
    ) = selected_participants_with_indices.into_iter().unzip();

    let j = participants
        .iter()
        .position(|p| p == my_account_id)
        .expect("Should exist") as u16;

    let i = sorted_selected_participants_indices
        .iter()
        .position(|p| p == &j)
        .map(|i| i as u16)
        .ok_or_else(|| gadget_common::Error::ParticipantNotSelected {
            id: *my_account_id,
            reason: String::from("we are not selected to sign"),
        })?;

    let user_id_to_account_id_mapping = sorted_selected_participants
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, p)| (i as UserID, p))
        .collect();
    Ok((
        i,
        sorted_selected_participants_indices,
        user_id_to_account_id_mapping,
    ))
}
