use crate::error::{Result, TangleEventListenerError};
use crate::events::{EventMatcher, TangleEvent, TangleResult};
use gadget_contexts::services::tangle::TangleClientContext;
use std::any::Any;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::services::events::{JobCalled, JobResultSubmitted};

pub async fn services_pre_processor<C: TangleClientContext, E: EventMatcher<Output: Clone>>(
    event: TangleEvent<C, E>,
) -> Result<TangleEvent<C, E>> {
    let TangleEvent {
        evt,
        mut context,
        block_number,
        signer,
        client,
        job_id,
        service_id,
        stopper,
        ..
    } = event;
    let boxed_item = Box::new(evt) as Box<dyn Any>;

    let (event_job_id, event_service_id, event_call_id, args) =
        if let Some(res) = boxed_item.downcast_ref::<JobCalled>() {
            (res.job, res.service_id, res.call_id, res.args.clone())
        } else if let Some(res) = boxed_item.downcast_ref::<JobResultSubmitted>() {
            (res.job, res.service_id, res.call_id, vec![])
        } else {
            return Err(TangleEventListenerError::SkipPreProcessedType);
        };

    gadget_logging::info!("Pre-processing event for service-id/job-id = {service_id}/{job_id} ...");

    if event_job_id == job_id && event_service_id == service_id {
        gadget_logging::info!(
            "Found actionable event for service-id/job-id = {event_service_id}/{event_job_id} ..."
        );
        // Set the call ID that way the user can access it in the job function
        context.set_call_id(event_call_id);
        return Ok(TangleEvent {
            evt: *boxed_item.downcast().unwrap(),
            context,
            call_id: Some(event_call_id),
            args,
            block_number,
            signer,
            client,
            job_id: event_job_id,
            service_id: event_service_id,
            stopper,
        });
    }

    Err(TangleEventListenerError::SkipPreProcessedType)
}

/// By default, the tangle post-processor takes in a job result and submits the result on-chain
pub async fn services_post_processor<R: serde::Serialize>(
    TangleResult {
        results,
        service_id,
        call_id,
        client,
        signer,
    }: TangleResult<R>,
) -> Result<()> {
    gadget_logging::info!(
        "Submitting result on-chain for service {service_id} call_id {call_id} ..."
    );
    let response = api::tx().services().submit_result(
        service_id,
        call_id,
        vec![gadget_blueprint_serde::to_field(results)?],
    );
    let _ = gadget_utils_tangle::tx::send(&client, &signer, &response)
        .await
        .map_err(|err| TangleEventListenerError::Client(err.to_string()))?;
    gadget_logging::info!("Submitted result on-chain");
    Ok(())
}
