pub mod extract;
use blueprint_sdk::JobCall;
pub use extract::*;

#[bon::builder]
pub fn create_call(job_id: u32, call_id: u64) -> JobCall {
    let mut call = JobCall::empty(job_id);
    call.metadata_mut()
        .insert(CallId::METADATA_KEY, call_id.into());
    call
}
