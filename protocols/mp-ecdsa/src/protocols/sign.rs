use crate::MpEcdsaProtocolConfig;
use async_trait::async_trait;
use gadget_core::job_manager::ProtocolWorkManager;
use std::error::Error;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::{Job, WebbGadgetProtocol};
use webb_gadget::{Block, BlockImportNotification, FinalityNotification};

pub struct MpEcdsaSigningProtocol {}

pub async fn create_protocol(
    _config: &MpEcdsaProtocolConfig,
) -> Result<MpEcdsaSigningProtocol, Box<dyn Error>> {
    Ok(MpEcdsaSigningProtocol {})
}

#[async_trait]
impl<B: Block> WebbGadgetProtocol<B> for MpEcdsaSigningProtocol {
    async fn get_next_job(
        &self,
        notification: &FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Job>, webb_gadget::Error> {
        todo!()
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
}
