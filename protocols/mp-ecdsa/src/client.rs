use crate::util::{find_authorities_change, DebugLogger};
use crate::MpEcdsaProtocolConfig;
use dkg_runtime_primitives::crypto::AuthorityId;
use dkg_runtime_primitives::{AuthoritySet, AuthoritySetId, DKGApi, MaxAuthorities, MaxProposalLength};
use gadget_core::gadget::substrate::Client;
use sc_client_api::{BlockchainEvents, FinalityNotifications, HeaderBackend, ImportNotifications};
use sp_api::{NumberFor, ProvideRuntimeApi};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use webb_gadget::{Backend, Block, BlockImportNotification, FinalityNotification, Header};
use crate::client::types::AnticipatedKeygenExecutionStatus;
use crate::keystore::DKGKeystore;

pub struct MpEcdsaClient<B, BE, C> {
    client: Arc<C>,
    key_store: DKGKeystore,
    finality_notification_stream: Arc<Mutex<FinalityNotifications<B>>>,
    block_import_notification_stream: Arc<Mutex<ImportNotifications<B>>>,
    latest_finality_notification: Arc<parking_lot::Mutex<Option<FinalityNotification<B>>>>,
    logger: DebugLogger,
    _block: std::marker::PhantomData<BE>,
}

pub async fn create_client<B: Block, BE: Backend<B>>(
    _config: &MpEcdsaProtocolConfig,
) -> Result<MpEcdsaClient<B, BE, ()>, Box<dyn Error>> {
    Ok(MpEcdsaClient {
        _block: std::marker::PhantomData,
    })
}

pub trait ClientWithApi<B, BE>:
    BlockchainEvents<B> + HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync
where
    B: Block,
    BE: Backend<B>,
    <Self as ProvideRuntimeApi<B>>::Api:
        DKGApi<B, AuthorityId, NumberFor<B>, MaxProposalLength, MaxAuthorities>,
{
}

impl<B, BE, T> ClientWithApi<B, BE> for T
where
    B: Block,
    BE: Backend<B>,
    T: BlockchainEvents<B> + HeaderBackend<B> + ProvideRuntimeApi<B> + Send + Sync,
    <T as ProvideRuntimeApi<B>>::Api:
        DKGApi<B, AuthorityId, NumberFor<B>, MaxProposalLength, MaxAuthorities>,
{
}

impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> MpEcdsaClient<B, BE, C> {
    pub async fn validator_set(
        &self,
        header: &B::Header,
    ) -> Option<(
        AuthoritySet<AuthorityId, MaxAuthorities>,
        AuthoritySet<AuthorityId, MaxAuthorities>,
    )> {
        let new = if let Some((new, queued)) = find_authorities_change::<B>(header) {
            Some((new, queued))
        } else {
            let at = header.hash();
            let current_authority_set = exec_client_function(&self.client, move |client| {
                client.runtime_api().authority_set(at).ok()
            })
            .await;

            let queued_authority_set = exec_client_function(&self.client, move |client| {
                client.runtime_api().queued_authority_set(at).ok()
            })
            .await;

            match (current_authority_set, queued_authority_set) {
                (Some(current), Some(queued)) => Some((current, queued)),
                _ => None,
            }
        };

        self.logger
            .trace(format!("🕸️  active validator set: {new:?}"));

        new
    }

    pub async fn should_execute_new_keygen(
        &self,
        header: &B::Header,
    ) -> AnticipatedKeygenExecutionStatus {
        // query runtime api to check if we should execute new keygen.
        let at = header.hash();
        let (execute, force_execute) = self
            .exec_client_function(move |client| {
                client.runtime_api().should_execute_new_keygen(at).unwrap_or_default()
            })
            .await;

        AnticipatedKeygenExecutionStatus { execute, force_execute }
    }

    /// Get the next DKG public key
    pub async fn get_next_dkg_pub_key(
        &self,
        header: &B::Header,
    ) -> Option<(AuthoritySetId, Vec<u8>)> {
        let at = header.hash();
        self.exec_client_function(move |client| {
            client.runtime_api().next_dkg_pub_key(at).unwrap_or_default()
        })
            .await
    }

    pub async fn dkg_pub_key_is_unset(&self, header: &B::Header) -> bool {
        self.get_dkg_pub_key(header).await.1.is_empty()
    }

    /// Get the active DKG public key
    pub async fn get_dkg_pub_key(&self, header: &B::Header) -> (AuthoritySetId, Vec<u8>) {
        let at = header.hash();
        self.exec_client_function(move |client| {
            client.runtime_api().dkg_pub_key(at).unwrap_or_default()
        })
            .await
    }

    /// Get the party index of our worker
    ///
    /// Returns `None` if we are not in the best authority set
    pub async fn get_party_index(&self, header: &B::Header) -> Option<u16> {
        let public = self.get_authority_public_key();
        let best_authorities = self.get_best_authorities(header).await;
        for elt in best_authorities {
            if elt.1 == public {
                return Some(elt.0)
            }
        }

        None
    }

    /// Get the best authorities for keygen
    pub async fn get_best_authorities(&self, header: &B::Header) -> Vec<(u16, AuthorityId)> {
        let at = header.hash();
        self.exec_client_function(move |client| {
            client.runtime_api().get_best_authorities(at).unwrap_or_default()
        })
            .await
    }

    /// Get the next party index of our worker for possible queued keygen
    ///
    /// Returns `None` if we are not in the next best authority set
    pub async fn get_next_party_index(&self, header: &B::Header) -> Option<u16> {
        let public = self.get_authority_public_key();
        let next_best_authorities = self.get_next_best_authorities(header).await;
        for elt in next_best_authorities {
            if elt.1 == public {
                return Some(elt.0)
            }
        }

        None
    }

    /// Get the next best authorities for keygen
    pub async fn get_next_best_authorities(&self, header: &B::Header) -> Vec<(u16, AuthorityId)> {
        let at = header.hash();
        self.exec_client_function(move |client| {
            client.runtime_api().get_next_best_authorities(at).unwrap_or_default()
        })
            .await
    }

    /// Get the signature threshold at a specific block
    pub async fn get_signature_threshold(&self, header: &B::Header) -> u16 {
        let at = header.hash();
        self.exec_client_function(move |client| {
            client.runtime_api().signature_threshold(at).unwrap_or_default()
        })
            .await
    }

    /// Get the next signature threshold at a specific block
    pub async fn get_next_signature_threshold(&self, header: &B::Header) -> u16 {
        let at = header.hash();
        self.exec_client_function(move |client| {
            client.runtime_api().next_signature_threshold(at).unwrap_or_default()
        })
            .await
    }

    pub fn get_authority_public_key(&self) -> AuthorityId {
        self.key_store
            .authority_id(
                &self.key_store.public_keys().expect("Could not find authority public key"),
            )
            .unwrap_or_else(|| panic!("Could not find authority public key"))
    }
}

impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> Client<B> for MpEcdsaClient<B, BE, C> {
    async fn get_next_finality_notification(&self) -> Option<FinalityNotification<B>> {
        let latest = self.finality_notification_stream.lock().await.next().await;
        *self.latest_finality_notification.lock().await = Some(latest.clone());
        latest
    }

    async fn get_latest_finality_notification(&self) -> Option<FinalityNotification<B>> {
        self.latest_finality_notification.lock().await.clone()
    }

    async fn get_next_block_import_notification(&self) -> Option<BlockImportNotification<B>> {
        self.block_import_notification_stream
            .lock()
            .await
            .next()
            .await
    }
}

async fn exec_client_function<C, F, T>(client: &Arc<C>, function: F) -> T
where
    for<'a> F: FnOnce(&'a C) -> T,
    C: Send + 'static,
    T: Send + 'static,
    F: Send + 'static,
{
    let client = client.clone();
    tokio::task::spawn_blocking(move || function(&client))
        .await
        .expect("Failed to spawn blocking task")
}

pub mod types {
    #[derive(Debug, Copy, Clone)]
    pub struct AnticipatedKeygenExecutionStatus {
        pub execute: bool,
        pub force_execute: bool,
    }
}