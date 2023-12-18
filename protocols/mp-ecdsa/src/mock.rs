// This file is part of Tangle.
// Copyright (C) 2022-2023 Webb Technologies Inc.
//
// Tangle is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Tangle is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Tangle.  If not, see <http://www.gnu.org/licenses/>.

use async_trait::async_trait;
use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Everything},
    PalletId,
};
use frame_system::EnsureSigned;
use gadget_common::client::AccountId;
use gadget_core::gadget::substrate::Client;
use sc_client_api::blockchain::{BlockStatus, Info};
use sc_client_api::{
    BlockchainEvents, FinalityNotification, FinalityNotifications, FinalizeSummary, HeaderBackend,
    ImportNotifications, StorageEventStream, StorageKey,
};
use sc_utils::mpsc::TracingUnboundedReceiver;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_application_crypto::RuntimePublic;
use sp_core::{Pair as PairT, H256};
use sp_runtime::{traits::Block as BlockT, traits::IdentityLookup, BuildStorage, DispatchResult};
use std::collections::HashMap;
pub type Balance = u128;
pub type BlockNumber = u64;

use crate::MpEcdsaProtocolConfig;
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WebbWorkManager;
use gadget_common::keystore::ECDSAKeyStore;
use gadget_common::Error;
use gadget_core::job_manager::WorkManagerInterface;
use sp_core::ecdsa;
use sp_core::ecdsa::Pair;
use sp_io::crypto::ecdsa_generate;
use sp_keystore::{testing::MemoryKeystore, KeystoreExt, KeystorePtr};
use sp_runtime::traits::NumberFor;
use sp_std::sync::Arc;
use tangle_primitives::{
    jobs::{
        DkgKeyType, JobKey, JobSubmission, JobType, JobWithResult, ReportValidatorOffence,
        RpcResponseJobsData, ValidatorOffenceType,
    },
    roles::{RoleTypeMetadata, TssRoleMetadata},
    traits::{
        jobs::{JobToFee, MPCHandler},
        roles::RolesHandler,
    },
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// Key type for DKG keys
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"wdkg");

type Block = frame_system::mocking::MockBlock<Runtime>;

impl frame_system::Config for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Block = Block;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type BlockWeights = ();
    type BlockLength = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = Everything;
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type MaxLocks = ();
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = ();
    type WeightInfo = ();
    type RuntimeHoldReason = RuntimeHoldReason;
    type MaxHolds = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ();
    type WeightInfo = ();
}

pub struct MockDKGPallet;

impl MockDKGPallet {
    fn job_to_fee(job: &JobSubmission<AccountId, BlockNumber>) -> Balance {
        if job.job_type.is_phase_one() {
            job.job_type
                .clone()
                .get_participants()
                .unwrap()
                .len()
                .try_into()
                .unwrap()
        } else {
            20
        }
    }
}

pub struct MockZkSaasPallet;
impl MockZkSaasPallet {
    fn job_to_fee(job: &JobSubmission<AccountId, BlockNumber>) -> Balance {
        if job.job_type.is_phase_one() {
            10
        } else {
            20
        }
    }
}

pub struct MockJobToFeeHandler;

impl JobToFee<AccountId, BlockNumber> for MockJobToFeeHandler {
    type Balance = Balance;

    fn job_to_fee(job: &JobSubmission<AccountId, BlockNumber>) -> Balance {
        match job.job_type {
            JobType::DKGTSSPhaseOne(_) => MockDKGPallet::job_to_fee(job),
            JobType::DKGTSSPhaseTwo(_) => MockDKGPallet::job_to_fee(job),
            JobType::ZkSaaSPhaseOne(_) => MockZkSaasPallet::job_to_fee(job),
            JobType::ZkSaaSPhaseTwo(_) => MockZkSaasPallet::job_to_fee(job),
        }
    }
}

pub struct MockRolesHandler;

impl RolesHandler<AccountId> for MockRolesHandler {
    fn is_validator(address: AccountId, _role_type: JobKey) -> bool {
        let validators = (0..8)
            .map(|i| {
                Pair::from_seed_slice(&id_to_seed(i))
                    .expect("Should exist")
                    .public()
            })
            .collect::<Vec<_>>();
        validators.contains(&address)
    }

    fn report_offence(_offence_report: ReportValidatorOffence<AccountId>) -> DispatchResult {
        Ok(())
    }

    fn get_validator_metadata(address: AccountId, _job_key: JobKey) -> Option<RoleTypeMetadata> {
        let mock_err_account = Pair::from_seed_slice(&id_to_seed(100))
            .expect("Should exist")
            .public();
        if address == mock_err_account {
            None
        } else {
            Some(RoleTypeMetadata::Tss(TssRoleMetadata {
                key_type: DkgKeyType::Ecdsa,
                authority_key: mock_pub_key().to_raw_vec(),
            }))
        }
    }
}

pub struct MockMPCHandler;

impl MPCHandler<AccountId, BlockNumber, Balance> for MockMPCHandler {
    fn verify(_data: JobWithResult<AccountId>) -> DispatchResult {
        Ok(())
    }

    fn verify_validator_report(
        _validator: AccountId,
        _offence: ValidatorOffenceType,
        _signatures: Vec<Vec<u8>>,
    ) -> DispatchResult {
        Ok(())
    }

    fn validate_authority_key(_validator: AccountId, _authority_key: Vec<u8>) -> DispatchResult {
        Ok(())
    }
}

parameter_types! {
    pub const JobsPalletId: PalletId = PalletId(*b"py/jobss");
}

impl pallet_jobs::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ForceOrigin = EnsureSigned<AccountId>;
    type Currency = Balances;
    type JobToFee = MockJobToFeeHandler;
    type RolesHandler = MockRolesHandler;
    type MPCHandler = MockMPCHandler;
    type PalletId = JobsPalletId;
    type WeightInfo = ();
}

construct_runtime!(
    pub enum Runtime
    {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        Jobs: pallet_jobs,
    }
);

#[async_trait::async_trait]
impl Client<Block> for Runtime {
    async fn get_next_finality_notification(&self) -> Option<FinalityNotification<Block>> {
        self.get_latest_finality_notification().await
    }

    async fn get_latest_finality_notification(&self) -> Option<FinalityNotification<Block>> {
        let (tx, rx) = sc_utils::mpsc::tracing_unbounded("mpsc_finality_notification", 999999);
        // forget rx so that it doesn't get dropped
        core::mem::forget(rx);
        let header = System::finalize();
        let summary = FinalizeSummary::<Block> {
            finalized: vec![header.hash()],
            header,
            stale_heads: vec![],
        };
        let notification = FinalityNotification::from_summary(summary, tx.clone());
        Some(notification)
    }
}

impl ProvideRuntimeApi<Block> for Runtime {
    type Api = Self;
    fn runtime_api(&self) -> ApiRef<Self::Api> {
        ApiRef::from(*self)
    }
}

impl<B: BlockT> BlockchainEvents<B> for Runtime {
    fn import_notification_stream(&self) -> ImportNotifications<B> {
        todo!()
    }

    fn every_import_notification_stream(&self) -> ImportNotifications<B> {
        todo!()
    }

    fn finality_notification_stream(&self) -> FinalityNotifications<B> {
        todo!()
    }

    fn storage_changes_notification_stream(
        &self,
        filter_keys: Option<&[StorageKey]>,
        child_filter_keys: Option<&[(StorageKey, Option<Vec<StorageKey>>)]>,
    ) -> sc_client_api::blockchain::Result<StorageEventStream<B::Hash>> {
        todo!()
    }
}

impl<B: BlockT> HeaderBackend<B> for Runtime {
    fn header(&self, hash: B::Hash) -> sc_client_api::blockchain::Result<Option<B::Header>> {
        todo!()
    }

    fn info(&self) -> Info<B> {
        todo!()
    }

    fn status(&self, hash: B::Hash) -> sc_client_api::blockchain::Result<BlockStatus> {
        todo!()
    }

    fn number(
        &self,
        hash: B::Hash,
    ) -> sc_client_api::blockchain::Result<Option<<<B as BlockT>::Header as sp_api::HeaderT>::Number>>
    {
        todo!()
    }

    fn hash(&self, number: NumberFor<B>) -> sc_client_api::blockchain::Result<Option<B::Hash>> {
        todo!()
    }
}

sp_api::mock_impl_runtime_apis! {
    impl pallet_jobs_rpc_runtime_api::JobsApi<Block, AccountId> for Runtime {
        fn query_jobs_by_validator(validator: AccountId) -> Result<Vec<RpcResponseJobsData<AccountId>>, String> { todo!() }
    }
}

pub struct ExtBuilder;

impl Default for ExtBuilder {
    fn default() -> Self {
        ExtBuilder
    }
}

pub fn id_to_seed(id: u8) -> [u8; 32] {
    [id; 32]
}

sp_externalities::decl_extension! {
    pub struct TracingUnboundedReceiverExt(TracingUnboundedReceiver<<Block as BlockT>::Hash>);
}

#[derive(Clone)]
pub struct MockNetwork {
    peers_tx: Arc<
        HashMap<
            AccountId,
            UnboundedSender<<WebbWorkManager as WorkManagerInterface>::ProtocolMessage>,
        >,
    >,
    peers_rx: Arc<
        HashMap<
            AccountId,
            tokio::sync::Mutex<
                UnboundedReceiver<<WebbWorkManager as WorkManagerInterface>::ProtocolMessage>,
            >,
        >,
    >,
    my_id: AccountId,
}

impl MockNetwork {
    pub fn setup(ids: &Vec<AccountId>) -> Vec<Self> {
        let mut peers_tx = HashMap::new();
        let mut peers_rx = HashMap::new();
        let mut networks = Vec::new();

        for id in ids {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            peers_tx.insert(id.clone(), tx);
            peers_rx.insert(id.clone(), tokio::sync::Mutex::new(rx));
        }

        let peers_tx = Arc::new(peers_tx);
        let peers_rx = Arc::new(peers_rx);

        for id in ids {
            let network = Self {
                peers_tx: peers_tx.clone(),
                peers_rx: peers_rx.clone(),
                my_id: id.clone(),
            };
            networks.push(network);
        }

        networks
    }
}

#[async_trait]
impl Network for MockNetwork {
    async fn next_message(
        &self,
    ) -> Option<<WebbWorkManager as WorkManagerInterface>::ProtocolMessage> {
        self.peers_rx.get(&self.my_id)?.lock().await.recv().await
    }

    async fn send_message(
        &self,
        message: <WebbWorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), Error> {
        let _check_message_has_ids = message
            .from_account_id
            .ok_or_else(|| Error::MissingNetworkId)?;
        if let Some(peer_id) = message.to_account_id {
            let tx = self
                .peers_tx
                .get(&peer_id)
                .ok_or(Error::PeerNotFound { id: peer_id })?;
            tx.send(message).map_err(|err| Error::NetworkError {
                err: err.to_string(),
            })?;
        } else {
            // Broadcast to everyone except ourself
            for (peer_id, tx) in self.peers_tx.iter() {
                if peer_id != &self.my_id {
                    tx.send(message.clone())
                        .map_err(|err| Error::NetworkError {
                            err: err.to_string(),
                        })?;
                }
            }
        }

        Ok(())
    }

    async fn run(&self) -> Result<(), Error> {
        Ok(())
    }
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext<const N: usize>() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    let identities = (0..N)
        .map(|i| Pair::from_seed_slice(&id_to_seed(i as u8)).expect("Should create keypair"))
        .collect::<Vec<_>>();

    let balances = identities
        .iter()
        .map(|pair| (pair.public(), 100u128))
        .collect::<Vec<_>>();

    let public_identities = identities.iter().map(|i| i.public()).collect::<Vec<_>>();
    let keygen_networks = MockNetwork::setup(&public_identities);
    let signing_networks = MockNetwork::setup(&public_identities);

    pallet_balances::GenesisConfig::<Runtime> { balances }
        .assimilate_storage(&mut t)
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    // set to block 1 to test events
    ext.execute_with(|| System::set_block_number(1));
    ext.register_extension(KeystoreExt(Arc::new(MemoryKeystore::new()) as KeystorePtr));

    let mock_client = Runtime;

    for (idx, ((identity, keygen_network), signing_network)) in identities
        .into_iter()
        .zip(keygen_networks)
        .zip(signing_networks)
        .enumerate()
    {
        let protocol_config = MpEcdsaProtocolConfig {
            account_id: identity.public(),
        };

        let logger = DebugLogger {
            peer_id: format!("Peer {idx}"),
        };

        let ecdsa_keystore = ECDSAKeyStore::in_memory();

        let task = async move {
            if let Err(err) = crate::run::<_, sc_client_api::in_mem::Backend<Block>, _, _, _, _>(
                protocol_config,
                mock_client.clone(),
                logger,
                ecdsa_keystore,
                keygen_network,
                signing_network,
            )
            .await
            {
                log::error!("Error running protocol: {err:?}");
            }
        };

        tokio::task::spawn(task);
    }

    ext
}

fn mock_pub_key() -> ecdsa::Public {
    ecdsa_generate(KEY_TYPE, None)
}
