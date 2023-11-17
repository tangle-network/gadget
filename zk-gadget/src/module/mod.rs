use crate::client_ext::ClientWithApi;
use crate::network::{RegistantId, ZkNetworkService};
use async_trait::async_trait;
use gadget_core::job_manager::ProtocolWorkManager;
use gadget_core::Block;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::WebbGadgetModule;
use webb_gadget::{BlockImportNotification, Error, FinalityNotification};

pub mod proto_gen;

pub struct ZkModule<T, C, B> {
    pub party_id: RegistantId,
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

        // TODO: call proto_gen::create_zk_async_protocol to generate a protocol, then
        // push the returned remote and protocol to the job manager. E.g.,:
        let task_id = [0u8; 32];
        let n_parties = 5; // TODO: get this from somewhere
        let (remote, protocol) = proto_gen::create_zk_async_protocol(
            0,
            now,
            0,
            task_id,
            self.party_id,
            n_parties,
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
