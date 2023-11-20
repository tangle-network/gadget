use crate::client_ext::ClientWithApi;
use crate::network::{RegistantId, ZkNetworkService};
use async_trait::async_trait;
use gadget_core::job_manager::ProtocolWorkManager;
use sp_runtime::traits::Block;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::WebbGadgetModule;
use webb_gadget::{BlockImportNotification, Error, FinalityNotification};

pub mod proto_gen;

pub struct ZkModule<T, C, B> {
    pub party_id: RegistantId,
    pub n_parties: usize,
    pub additional_protocol_params: T,
    pub client: C,
    pub network: ZkNetworkService,
    pub async_protocol_generator:
        Box<dyn proto_gen::AsyncProtocolGenerator<T, Error, ZkNetworkService, C, B>>,
}

pub trait AdditionalProtocolParams: Send + Sync + Clone + 'static {}
impl<T: Send + Sync + Clone + 'static> AdditionalProtocolParams for T {}

#[async_trait]
impl<B: Block, T: AdditionalProtocolParams, C: ClientWithApi<B>> WebbGadgetModule<B>
    for ZkModule<T, C, B>
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
            let (remote, protocol) = proto_gen::create_zk_async_protocol(
                now / 6000,
                now,
                0,
                task_id,
                self.party_id,
                self.n_parties,
                self.additional_protocol_params.clone(),
                self.network.clone(),
                self.client.clone(),
                &*self.async_protocol_generator,
            );

            if let Err(err) =
                _job_manager.push_task(task_id, false, std::sync::Arc::new(remote), protocol)
            {
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
