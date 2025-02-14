use blueprint_sdk::JobCall;
use bytes::Bytes;
use gadget_blueprint_serde::Field;
use tangle_subxt::parity_scale_codec::Encode;
use tangle_subxt::subxt::utils::AccountId32;

pub mod extract;
pub mod layers;
pub mod producer;

#[bon::builder]
pub fn create_call(
    job_id: u32,
    call_id: u64,
    block_number: u64,
    args: Option<Field<AccountId32>>,
) -> JobCall {
    let mut call = match args {
        Some(args) => JobCall::new(job_id, Bytes::from(args.encode())),
        None => JobCall::empty(job_id),
    };
    call.metadata_mut()
        .insert(extract::CallId::METADATA_KEY, call_id);

    call.metadata_mut()
        .insert(extract::BlockNumber::METADATA_KEY, block_number);
    call
}
