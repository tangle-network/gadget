use crate::clients::tangle::runtime::TangleClient;
use crate::event_listener::tangle::TangleEvent;
use crate::Error;
use subxt_core::events::StaticEvent;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::call::{Job, ServiceId};
use tangle_subxt::tangle_testnet_runtime::api::services::events::{
    job_called, JobCalled, JobResultSubmitted,
};

pub trait ServicesJobPalletItem: StaticEvent {
    fn call_id(&self) -> job_called::CallId;
    fn job_id(&self) -> Job;
    fn service_id(&self) -> ServiceId;
    fn args(self) -> Option<job_called::Args> {
        None
    }
}

impl ServicesJobPalletItem for JobCalled {
    fn call_id(&self) -> job_called::CallId {
        self.call_id
    }

    fn job_id(&self) -> Job {
        self.job
    }

    fn service_id(&self) -> ServiceId {
        self.service_id
    }

    fn args(self) -> Option<job_called::Args> {
        Some(self.args.clone())
    }
}

impl ServicesJobPalletItem for JobResultSubmitted {
    fn call_id(&self) -> job_called::CallId {
        self.call_id
    }

    fn job_id(&self) -> Job {
        self.job
    }

    fn service_id(&self) -> ServiceId {
        self.service_id
    }
}

pub async fn services_pre_processor<Ctx, Event: ServicesJobPalletItem>(
    mut event: TangleEvent<Ctx, Event>,
) -> Result<TangleEvent<Ctx, Event>, Error> {
    let this_service_id = event.service_id;
    let this_job_id = event.job_id;
    crate::info!("Pre-processing event for sid/bid = {this_service_id}/{this_job_id} ...");
    if let Ok(Some(evt)) = event.evt.as_event::<Event>() {
        let service_id = evt.service_id();
        let job = evt.job_id();
        let call_id = evt.call_id();
        let args = evt.args().unwrap_or_default();

        if job == this_job_id && service_id == this_service_id {
            crate::info!("Found actionable event for sid/bid = {service_id}/{job} ...");
            event.call_id = Some(call_id);
            event.args = args;
            return Ok(event);
        }
    }

    Err(Error::SkipPreProcessedType)
}

/// By default, the tangle post-processor takes in a job result and submits the result on-chain
pub async fn services_post_processor(
    TangleJobResult {
        results,
        service_id,
        call_id,
        client,
        signer,
    }: TangleJobResult,
) -> Result<(), Error> {
    crate::info!("Submitting result on-chain for service {service_id} call_id {call_id} ...");
    let response = api::tx()
        .services()
        .submit_result(service_id, call_id, results);
    let _ = crate::tx::tangle::send(&client, &signer, &response)
        .await
        .map_err(|err| Error::Client(err.to_string()))?;
    crate::info!("Submitted result on-chain");
    Ok(())
}

pub struct TangleJobResult {
    pub results: types::submit_result::Result,
    pub service_id: ServiceId,
    pub call_id: job_called::CallId,
    pub client: TangleClient,
    pub signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
}
