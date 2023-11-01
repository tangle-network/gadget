use async_trait::async_trait;
use gadget_core::job_manager::ProtocolWorkManager;
use gadget_core::Block;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::WebbGadgetModule;
use webb_gadget::{BlockImportNotification, Error, FinalityNotification};

pub struct ZkModule {
    #[allow(dead_code)]
    pub job_manager: Option<ProtocolWorkManager<WebbWorkManager>>,
}

#[async_trait]
impl<B: Block> WebbGadgetModule<B> for ZkModule {
    fn on_job_manager_created(&mut self, job_manager: ProtocolWorkManager<WebbWorkManager>) {
        self.job_manager = Some(job_manager);
    }

    async fn process_finality_notification(
        &self,
        _notification: FinalityNotification<B>,
    ) -> Result<(), Error> {
        todo!()
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
    ) -> Result<(), Error> {
        todo!()
    }

    async fn process_error(&self, _error: Error) {
        todo!()
    }
}
