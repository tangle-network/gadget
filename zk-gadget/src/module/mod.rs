use async_trait::async_trait;
use gadget_core::job_manager::ProtocolWorkManager;
use gadget_core::Block;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::WebbGadgetModule;
use webb_gadget::{BlockImportNotification, Error, FinalityNotification};

pub struct ZkModule {}

#[async_trait]
impl<B: Block> WebbGadgetModule<B> for ZkModule {
    async fn process_finality_notification(
        &self,
        _notification: FinalityNotification<B>,
        _now: u64,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), Error> {
        todo!()
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), Error> {
        todo!()
    }

    async fn process_error(
        &self,
        _error: Error,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) {
        todo!()
    }
}
