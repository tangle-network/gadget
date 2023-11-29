use crate::MpEcdsaProtocolConfig;
use async_trait::async_trait;
use gadget_core::job_manager::ProtocolWorkManager;
use std::error::Error;
use sc_client_api::Backend;
use tangle_primitives::jobs::JobType;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::{Job, WebbGadgetProtocol, WorkManagerConfig};
use webb_gadget::{Block, BlockImportNotification, FinalityNotification};
use crate::client::{AccountId, ClientWithApi, MpEcdsaClient};

pub struct MpEcdsaKeygenProtocol<B, BE, C> {
    client: MpEcdsaClient<B, BE, C>,
    account_id: AccountId,
}

pub async fn create_protocol(
    _config: &MpEcdsaProtocolConfig,
) -> Result<MpEcdsaKeygenProtocol, Box<dyn Error>> {
    Ok(MpEcdsaKeygenProtocol {})
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> WebbGadgetProtocol<B> for MpEcdsaKeygenProtocol<B, BE, C> {
    async fn get_next_jobs(
        &self,
        _notification: &FinalityNotification<B>,
        _now: u64,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Vec<Job>>, webb_gadget::Error> {
        let jobs = self.client.query_jobs_by_validator(self.account_id).await?
            .into_iter()
            .filter(|r| matches!(r.job_type, JobType::DKG(..)));

        let mut ret = vec![];

        for job in jobs {
            let participants = job.job_type.get_participants().expect("Should exist for DKG");
            if participants.contains(&self.account_id) {
                // Create a job, push to ret

            }
        }

        Ok(Some(ret))
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
