use crate::network::RegistantId;
use async_trait::async_trait;
use gadget_core::job_manager::ProtocolWorkManager;
use gadget_core::Block;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::WebbGadgetModule;
use webb_gadget::{BlockImportNotification, Error, FinalityNotification};

pub mod proto_gen;

pub struct ZkModule {
    pub party_id: RegistantId,
}

#[async_trait]
impl<B: Block> WebbGadgetModule<B> for ZkModule {
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
        // TODO: call proto_gen::create_zk_async_protocol to generate a protocol, then
        // push the returned remote and protocol to the job manager
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
        _error: Error,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) {
        todo!()
    }
}
