use auto_impl::auto_impl;
use gadget_common::async_trait::async_trait;
use gadget_common::config::DebugLogger;
use gadget_common::tangle_runtime::api;
use gadget_common::tangle_subxt::subxt;
use gadget_common::tangle_subxt::subxt::tx::TxPayload;
use gadget_common::tangle_subxt::subxt::OnlineClient;
use std::fmt::Debug;

#[async_trait]
#[auto_impl(Arc)]
pub trait TanglePalletSubmitter: Send + Sync + Debug + 'static {
    async fn submit_service_result(
        &self,
        service_id: u64,
        call_id: u64,
        result: api::services::calls::types::submit_result::Result,
    ) -> Result<(), gadget_common::Error>;
}

pub struct SubxtPalletSubmitter<C, S>
where
    C: subxt::Config,
    S: subxt::tx::Signer<C>,
{
    subxt_client: OnlineClient<C>,
    signer: S,
    logger: DebugLogger,
}

impl<C: subxt::Config, S: subxt::tx::Signer<C>> Debug for SubxtPalletSubmitter<C, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubxtPalletSubmitter")
            .field("signer", &self.signer.account_id())
            .finish()
    }
}

#[async_trait]
impl<C, S> TanglePalletSubmitter for SubxtPalletSubmitter<C, S>
where
    C: subxt::Config + Send + Sync + 'static,
    S: subxt::tx::Signer<C> + Send + Sync + 'static,
    C::AccountId: std::fmt::Display + Send + Sync + 'static,
    C::Hash: std::fmt::Display,
    <C::ExtrinsicParams as subxt::config::ExtrinsicParams<C>>::OtherParams:
        Default + Send + Sync + 'static,
{
    async fn submit_service_result(
        &self,
        service_id: u64,
        call_id: u64,
        result: api::services::calls::types::submit_result::Result,
    ) -> Result<(), gadget_common::Error> {
        let tx = api::tx()
            .services()
            .submit_result(service_id, call_id, result);
        match self.submit(&tx).await {
            Ok(hash) => {
                self.logger.info(format!(
                    "({}) Service result submitted for service-id/call-id: {service_id}/{call_id} at block: {hash}",
                    self.signer.account_id(),
                ));
                Ok(())
            }
            Err(err) if err.to_string().contains("JobNotFound") => {
                self.logger.warn(format!(
                    "({}) Service not found for service-id/call-id: {service_id}/{call_id}",
                    self.signer.account_id(),
                ));
                Ok(())
            }
            Err(err) => {
                return Err(gadget_common::Error::ClientError {
                    err: format!("Failed to submit job result: {err:?}"),
                })
            }
        }
    }
}

impl<C, S> SubxtPalletSubmitter<C, S>
where
    C: subxt::Config,
    C::AccountId: std::fmt::Display,
    S: subxt::tx::Signer<C>,
    C::Hash: std::fmt::Display,
    <C::ExtrinsicParams as subxt::config::ExtrinsicParams<C>>::OtherParams: Default,
{
    pub async fn new(signer: S, logger: DebugLogger) -> Result<Self, gadget_common::Error> {
        let subxt_client =
            OnlineClient::<C>::new()
                .await
                .map_err(|err| gadget_common::Error::ClientError {
                    err: format!("Failed to setup api: {err:?}"),
                })?;
        Ok(Self::with_client(subxt_client, signer, logger))
    }

    pub fn with_client(subxt_client: OnlineClient<C>, signer: S, logger: DebugLogger) -> Self {
        Self {
            subxt_client,
            signer,
            logger,
        }
    }

    async fn submit<Call: TxPayload>(&self, call: &Call) -> anyhow::Result<C::Hash> {
        if let Some(details) = call.validation_details() {
            self.logger.trace(format!(
                "({}) Submitting {}.{}",
                self.signer.account_id(),
                details.pallet_name,
                details.call_name
            ));
        }
        Ok(self
            .subxt_client
            .tx()
            .sign_and_submit_then_watch_default(call, &self.signer)
            .await?
            .wait_for_finalized_success()
            .await?
            .block_hash())
    }
}

#[cfg(test)]
#[cfg(not(target_family = "wasm"))]
mod tests {

    use crate::transaction_manager::tangle::SubxtPalletSubmitter;
    use gadget_common::prelude::*;
    use gadget_common::subxt_signer;
    use gadget_common::tangle_subxt::{
        subxt::{tx::Signer, utils::AccountId32, PolkadotConfig},
        tangle_testnet_runtime::api,
        tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec,
    };

    #[gadget_io::tokio::test]
    #[ignore = "This test requires a running general node"]
    async fn subxt_pallet_submitter() -> anyhow::Result<()> {
        let logger = DebugLogger { id: "test".into() };
        let alice = subxt_signer::sr25519::dev::alice();
        let bob = subxt_signer::sr25519::dev::bob();
        let alice_account_id =
            <subxt_signer::sr25519::Keypair as Signer<PolkadotConfig>>::account_id(&alice);
        let bob_account_id =
            <subxt_signer::sr25519::Keypair as Signer<PolkadotConfig>>::account_id(&bob);
        let tx_manager =
            SubxtPalletSubmitter::<PolkadotConfig, _>::new(alice.clone(), logger).await?;
        let dkg_phase_one = jobs::JobSubmission {
            expiry: 100u64,
            ttl: 100u64,
            fallback: jobs::FallbackOptions::Destroy,
            job_type: jobs::JobType::DKGTSSPhaseOne(jobs::tss::DKGTSSPhaseOneJobType {
                participants: BoundedVec::<AccountId32>(vec![alice_account_id, bob_account_id]),
                threshold: 1u8,
                permitted_caller: None,
                role_type: roles::tss::ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1,
                hd_wallet: false,
                __ignore: Default::default(),
            }),
        };
        let tx = api::tx().jobs().submit_job(dkg_phase_one);
        let _hash = tx_manager.submit(&tx).await?;
        Ok(())
    }
}
