use crate::MpEcdsaProtocolConfig;
use async_trait::async_trait;
use gadget_core::job_manager::ProtocolWorkManager;
use std::error::Error;
use sc_client_api::Backend;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::{Job, WebbGadgetProtocol, WorkManagerConfig};
use webb_gadget::{Block, BlockImportNotification, FinalityNotification};
use crate::client::ClientWithApi;
use crate::protocols::managers::keygen::KeygenManager;

pub struct MpEcdsaKeygenProtocol<B, BE, C> {
    keygen_manager: KeygenManager<B, BE, C>
}

pub async fn create_protocol(
    _config: &MpEcdsaProtocolConfig,
) -> Result<MpEcdsaKeygenProtocol, Box<dyn Error>> {
    Ok(MpEcdsaKeygenProtocol {})
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> WebbGadgetProtocol<B> for MpEcdsaKeygenProtocol<B, BE, C> {
    async fn get_next_job(
        &self,
        notification: &FinalityNotification<B>,
        _now: u64,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Job>, webb_gadget::Error> {
        // Let the KeygenManager decide what to do next. The KeygenManager will return a job if it
        // wants to do something, or None if it doesn't want to do anything.
        Ok(self.keygen_manager.on_block_finalized(&notification.header, job_manager).await)
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), webb_gadget::Error> {
        Ok(())
    }

    async fn process_error(
        &self,
        error: webb_gadget::Error,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) {
        log::error!(target: "mp-ecdsa", "Error: {error:?}");
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: None, // Manual polling
            max_active_tasks: 1,
            max_pending_tasks: 1,
        }
    }
}
