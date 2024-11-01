use crate::event_listener::tangle::{EventMatcher, TangleEvent, TangleResult, ValueIntoFieldType};
use crate::Error;
use std::any::Any;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::services::events::{JobCalled, JobResultSubmitted};

pub async fn services_pre_processor<C, E: EventMatcher<Output: Clone>>(
    mut event: TangleEvent<C, E>,
) -> Result<TangleEvent<C, E>, Error> {
    let boxed_item = Box::new(event.evt.clone()) as Box<dyn Any>;

    let (job, service_id, call_id, args) = if let Some(res) = boxed_item.downcast_ref::<JobCalled>()
    {
        (res.job, res.service_id, res.call_id, res.args.clone())
    } else if let Some(res) = boxed_item.downcast_ref::<JobResultSubmitted>() {
        (res.job, res.service_id, res.call_id, vec![])
    } else {
        return Err(Error::SkipPreProcessedType);
    };

    let this_service_id = event.service_id;
    let this_job_id = event.job_id;
    crate::info!("Pre-processing event for sid/bid = {this_service_id}/{this_job_id} ...");

    if job == this_job_id && service_id == this_service_id {
        crate::info!("Found actionable event for sid/bid = {service_id}/{job} ...");
        event.call_id = Some(call_id);
        event.args = args;
        return Ok(event);
    }

    Err(Error::SkipPreProcessedType)
}

/// By default, the tangle post-processor takes in a job result and submits the result on-chain
pub async fn services_post_processor<Res: ValueIntoFieldType>(
    TangleResult {
        results,
        service_id,
        call_id,
        client,
        signer,
    }: TangleResult<Res>,
) -> Result<(), Error> {
    crate::info!("Submitting result on-chain for service {service_id} call_id {call_id} ...");
    let response =
        api::tx()
            .services()
            .submit_result(service_id, call_id, vec![results.into_field_type()]);
    let _ = crate::tx::tangle::send(&client, &signer, &response)
        .await
        .map_err(|err| Error::Client(err.to_string()))?;
    crate::info!("Submitted result on-chain");
    Ok(())
}
