use crate::client_ext::ClientWithApi;
use crate::network::RegistantId;
use async_trait::async_trait;
use gadget_core::job_manager::ProtocolWorkManager;
use sp_runtime::traits::Block;
use std::sync::Arc;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::WebbGadgetModule;
use webb_gadget::protocol::AsyncProtocol;
use webb_gadget::{BlockImportNotification, Error, FinalityNotification};

pub mod proto_gen;

pub struct ZkModule<M, B, C> {
    pub party_id: RegistantId,
    pub protocol: M,
    pub client: C,
    pub _pd: std::marker::PhantomData<B>,
}

pub trait AdditionalProtocolParams: Send + Sync + Clone + 'static {}
impl<T: Send + Sync + Clone + 'static> AdditionalProtocolParams for T {}

#[async_trait]
impl<M: AsyncProtocol + Send + Sync, B: Block, C: ClientWithApi<B>> WebbGadgetModule<B>
    for ZkModule<M, B, C>
{
    async fn process_finality_notification(
        &self,
        _notification: FinalityNotification<B>,
        now: u64,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), Error> {
        log::info!(
            "Party {} received a finality notification at {now}",
            self.party_id
        );

        if let Some(job) = self.client.get_next_job().await? {
            log::debug!("Found a job {} at {now}", job.job_id);
            let mut task_id = [0u8; 32];
            task_id[..8].copy_from_slice(&job.job_id.to_be_bytes()[..]);
            let (remote, protocol) = self
                .protocol
                .create(now / 6000, now, 0, task_id)
                .await
                .map_err(|err| Error::ClientError { err: err.reason })?;

            if let Err(err) = _job_manager.push_task(task_id, false, Arc::new(remote), protocol) {
                log::error!("Failed to push task to job manager: {err:?}");
            }
        } else {
            log::debug!("No Jobs found at #{now}");
        }
        Ok(())
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
        log::error!("Party {} received an error: {:?}", self.party_id, error);
    }
}
