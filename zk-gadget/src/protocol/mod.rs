use crate::client_ext::ClientWithApi;
use crate::network::RegistantId;
use async_trait::async_trait;
use gadget_core::job_manager::ProtocolWorkManager;
use sp_runtime::traits::Block;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::{Job, WebbGadgetProtocol};
use webb_gadget::protocol::AsyncProtocol;
use webb_gadget::{BlockImportNotification, Error, FinalityNotification};

pub mod proto_gen;

pub struct ZkProtocol<M, B, C> {
    pub party_id: RegistantId,
    pub protocol: M,
    pub client: C,
    pub _pd: std::marker::PhantomData<B>,
}

pub trait AdditionalProtocolParams: Send + Sync + Clone + 'static {}
impl<T: Send + Sync + Clone + 'static> AdditionalProtocolParams for T {}

#[async_trait]
impl<M: AsyncProtocol + Send + Sync, B: Block, C: ClientWithApi<B>> WebbGadgetProtocol<B>
    for ZkProtocol<M, B, C>
{
    async fn get_next_job(
        &self,
        _notification: &FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Job>, Error> {
        log::info!(
            "Party {} received a finality notification at {now}",
            self.party_id
        );

        if let Some(job) = self.client.get_next_job().await? {
            log::debug!("Found a job {} at {now}", job.job_id);
            let mut task_id = [0u8; 32];
            task_id[..8].copy_from_slice(&job.job_id.to_be_bytes()[..]);

            if job_manager.job_exists(&task_id) {
                return Ok(None);
            }

            let (remote, protocol) = self
                .protocol
                .create(now / 6000, now, 0, task_id)
                .await
                .map_err(|err| Error::ClientError { err: err.reason })?;

            return Ok(Some((remote, protocol)));
        } else {
            log::debug!("No Jobs found at #{now}");
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
        log::error!("Party {} received an error: {:?}", self.party_id, error);
    }
}
