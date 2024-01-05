use crate::client_ext::ClientWithApi;
use crate::network::RegistantId;
use async_trait::async_trait;
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::work_manager::WebbWorkManager;
use gadget_common::gadget::{Job, WebbGadgetProtocol};
use gadget_common::protocol::AsyncProtocol;
use gadget_common::{BlockImportNotification, Error, FinalityNotification};
use gadget_core::job_manager::ProtocolWorkManager;
use sp_runtime::traits::Block;

pub mod proto_gen;

pub struct ZkProtocol<M: AsyncProtocol, B, C> {
    pub party_id: RegistantId,
    pub protocol: M,
    pub additional_params: <M as AsyncProtocol>::AdditionalParams,
    pub logger: DebugLogger,
    pub client: C,
    pub _pd: std::marker::PhantomData<B>,
}

pub trait AdditionalProtocolParams: Send + Sync + Clone + 'static {}
impl<T: Send + Sync + Clone + 'static> AdditionalProtocolParams for T {}

#[async_trait]
impl<M: AsyncProtocol + Send + Sync, B: Block, C: ClientWithApi<B>> WebbGadgetProtocol<B>
    for ZkProtocol<M, B, C>
where
    <M as AsyncProtocol>::AdditionalParams: Clone,
{
    async fn get_next_jobs(
        &self,
        _notification: &FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Vec<Job>>, Error> {
        self.logger.info(format!(
            "Party {} received a finality notification at {now}",
            self.party_id
        ));

        if let Some(job) = self.client.get_next_job().await? {
            self.logger
                .debug(format!("Found a job {} at {now}", job.job_id));
            let mut task_id = [0u8; 32];
            task_id[..8].copy_from_slice(&job.job_id.to_be_bytes()[..]);

            if job_manager.job_exists(&task_id) {
                return Ok(None);
            }

            let (remote, protocol) = self
                .protocol
                .create(now / 6000, now, 0, task_id, self.additional_params.clone())
                .await
                .map_err(|err| Error::ClientError { err: err.reason })?;

            return Ok(Some(vec![(remote, protocol)]));
        } else {
            self.logger.debug(format!("No Jobs found at #{now}"));
        }

        Ok(None)
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn process_error(
        &self,
        error: Error,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) {
        self.logger.error(format!(
            "Party {} received an error: {:?}",
            self.party_id, error
        ));
    }

    fn logger(&self) -> &DebugLogger {
        &self.logger
    }
}
