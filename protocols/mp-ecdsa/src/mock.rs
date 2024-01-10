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
use frame_support::traits::Hooks;
use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Everything},
    PalletId,
};
use frame_system::EnsureSigned;
use gadget_common::client::AccountId;
use pallet_jobs_rpc_runtime_api::BlockNumberOf;
use sc_client_api::{FinalityNotification, FinalizeSummary};
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_core::{ByteArray, Pair, H256};
use sp_runtime::{traits::Block as BlockT, traits::IdentityLookup, BuildStorage, DispatchResult};
use std::collections::HashMap;
use std::time::Duration;

pub type Balance = u128;
pub type BlockNumber = u64;

use crate::mock::mock_wrapper_client::TestExternalitiesPalletSubmitter;
use crate::MpEcdsaProtocolConfig;
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WebbWorkManager;
use gadget_common::keystore::ECDSAKeyStore;
use gadget_common::locks::TokioMutexExt;
use gadget_common::Error;
use gadget_core::job_manager::WorkManagerInterface;
use sp_core::ecdsa;
use sp_io::crypto::ecdsa_generate;
use sp_keystore::{testing::MemoryKeystore, KeystoreExt, KeystorePtr};
use sp_std::sync::Arc;
use tangle_primitives::jobs::traits::{JobToFee, MPCHandler};
use tangle_primitives::jobs::{
    JobId, JobSubmission, JobType, JobWithResult, ReportValidatorOffence, RpcResponseJobsData,
    RpcResponsePhaseOneResult, ValidatorOffenceType,
};
use tangle_primitives::roles::traits::RolesHandler;
use tangle_primitives::roles::RoleType;
use test_utils::sync::substrate_test_channel::MultiThreadedTestExternalities;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// Key type for DKG keys
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"role");

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
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type ReserveIdentifier = ();
    type RuntimeHoldReason = RuntimeHoldReason;
    type FreezeIdentifier = ();
    type MaxLocks = ();
    type MaxReserves = ConstU32<50>;
    type MaxHolds = ();
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
    fn is_validator(address: AccountId, _role_type: RoleType) -> bool {
        let validators = (0..8).map(id_to_public).collect::<Vec<_>>();
        validators.contains(&address)
    }

    fn report_offence(_offence_report: ReportValidatorOffence<AccountId>) -> DispatchResult {
        Ok(())
    }

    fn get_validator_role_key(address: AccountId) -> Option<Vec<u8>> {
        let validators = (0..8).map(id_to_public).collect::<Vec<_>>();
        if validators.contains(&address) {
            Some(mock_pub_key().to_raw_vec())
        } else {
            None
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
    type Currency = Balances;
    type JobToFee = MockJobToFeeHandler;
    type RolesHandler = MockRolesHandler;
    type MPCHandler = MockMPCHandler;
    type ForceOrigin = EnsureSigned<AccountId>;
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

sp_api::mock_impl_runtime_apis! {
    impl pallet_jobs_rpc_runtime_api::JobsApi<Block, AccountId> for Runtime {
        fn query_jobs_by_validator(&self, validator: AccountId) -> Option<Vec<RpcResponseJobsData<AccountId, BlockNumberOf<Block>>>> {
            TEST_EXTERNALITIES.lock().as_ref().unwrap().execute_with(move || {
                Jobs::query_jobs_by_validator(validator)
            })
        }

        fn query_phase_one_by_id(
            &self,
            role_type: RoleType,
            job_id: JobId,
        ) -> Option<RpcResponsePhaseOneResult<AccountId, BlockNumberOf<Block>>> {
            TEST_EXTERNALITIES.lock().as_ref().unwrap().execute_with(move || {
                Jobs::query_phase_one_by_id(role_type, job_id)
            })
        }

        fn query_job_by_id(
            &self,
            role_type: RoleType,
            job_id: JobId,
        ) -> Option<RpcResponseJobsData<AccountId, BlockNumberOf<Block>>> {
            TEST_EXTERNALITIES.lock().as_ref().unwrap().execute_with(move || {
                Jobs::query_job_by_id(role_type, job_id)
            })
        }
    }
}

pub struct ExtBuilder;

impl Default for ExtBuilder {
    fn default() -> Self {
        ExtBuilder
    }
}

impl ProvideRuntimeApi<Block> for Runtime {
    type Api = Self;
    fn runtime_api(&self) -> ApiRef<Self::Api> {
        ApiRef::from(*self)
    }
}

pub fn id_to_pair(id: u8) -> ecdsa::Pair {
    ecdsa::Pair::from_string(&format!("//{id}//password"), None).unwrap()
}

pub fn id_to_public(id: u8) -> ecdsa::Public {
    id_to_pair(id).public()
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
            peers_tx.insert(*id, tx);
            peers_rx.insert(*id, tokio::sync::Mutex::new(rx));
        }

        let peers_tx = Arc::new(peers_tx);
        let peers_rx = Arc::new(peers_rx);

        for id in ids {
            let network = Self {
                peers_tx: peers_tx.clone(),
                peers_rx: peers_rx.clone(),
                my_id: *id,
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
        self.peers_rx
            .get(&self.my_id)?
            .lock_timeout(Duration::from_millis(500))
            .await
            .recv()
            .await
    }

    async fn send_message(
        &self,
        message: <WebbWorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), Error> {
        let _check_message_has_ids = message.from_network_id.ok_or(Error::MissingNetworkId)?;
        if let Some(peer_id) = message.to_network_id {
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
}

pub type MockBackend = sc_client_api::in_mem::Backend<Block>;

static TEST_EXTERNALITIES: parking_lot::Mutex<Option<MultiThreadedTestExternalities>> =
    parking_lot::Mutex::new(None);

pub fn advance_to_block(block_number: u64) {
    while System::block_number() < block_number {
        System::on_finalize(System::block_number());
        Jobs::on_finalize(System::block_number());
        Balances::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        Jobs::on_initialize(System::block_number());
        Balances::on_initialize(System::block_number());
    }
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub async fn new_test_ext<const N: usize>() -> MultiThreadedTestExternalities {
    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    let pairs = (0..N).map(|i| id_to_pair(i as u8)).collect::<Vec<_>>();
    let identities = pairs.iter().map(|pair| pair.public()).collect::<Vec<_>>();

    let balances = identities
        .iter()
        .map(|public| (*public, 100u128))
        .collect::<Vec<_>>();

    let keygen_networks = MockNetwork::setup(&identities);
    let signing_networks = MockNetwork::setup(&identities);

    pallet_balances::GenesisConfig::<Runtime> { balances }
        .assimilate_storage(&mut t)
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    /*ext.execute_with(|| System::set_block_number(1));
    ext.execute_with(|| System::on_finalize(1));
    ext.execute_with(|| Jobs::on_finalize(1));
    ext.execute_with(|| Balances::on_finalize(1));*/

    ext.register_extension(KeystoreExt(Arc::new(MemoryKeystore::new()) as KeystorePtr));

    let ext = MultiThreadedTestExternalities::new(ext);
    assert!(TEST_EXTERNALITIES.lock().replace(ext.clone()).is_none(), "Make sure to run tests serially with -- --test-threads=1 or with nextest to ensure separate program spaces per test");

    let finality_notification_txs = Arc::new(parking_lot::Mutex::new(Vec::<
        TracingUnboundedSender<FinalityNotification<Block>>,
    >::new()));
    let sinks = finality_notification_txs.clone();
    let externalities = ext.clone();

    // Spawn a thread that sends a finality notification whenever it detects a change in block number
    tokio::task::spawn(async move {
        let mut prev: Option<u64> = None;
        loop {
            let number = externalities
                .execute_with_async(move || {
                    let number = System::block_number();
                    System::finalize();
                    advance_to_block(number + 1);
                    number + 1
                })
                .await;
            // log::info!(target: "gadget", "Current block number: {number}");
            if prev.is_none() || prev.unwrap() != number {
                prev = Some(number);
                log::info!(target: "gadget", "Creating finality notification {number}");

                let lock = sinks.lock();
                for sink in lock.iter() {
                    let (faux_sink, faux_stream) = tracing_unbounded("faux_sink", 1024);
                    std::mem::forget(faux_stream);

                    let header = <Block as BlockT>::Header::new_from_number(number);
                    let summary = FinalizeSummary::<Block> {
                        finalized: vec![header.hash()],
                        header,
                        stale_heads: vec![],
                    };

                    let notification = FinalityNotification::from_summary(summary, faux_sink);
                    if sink.unbounded_send(notification).is_err() {
                        log::warn!(target: "gadget", "Will not deliver FinalityNotification because the receiver is gone");
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(12000)).await;
        }
    });

    for (idx, ((identity_pair, keygen_network), signing_network)) in pairs
        .into_iter()
        .zip(keygen_networks)
        .zip(signing_networks)
        .enumerate()
    {
        let mock_client_keygen =
            mock_wrapper_client::MockClient::new(Runtime, finality_notification_txs.clone()).await;
        let mock_client_signing =
            mock_wrapper_client::MockClient::new(Runtime, finality_notification_txs.clone()).await;

        let account_id = identity_pair.public();
        let protocol_config = MpEcdsaProtocolConfig { account_id };

        let logger = DebugLogger {
            peer_id: format!("Peer {idx}"),
        };

        let pallet_tx = TestExternalitiesPalletSubmitter {
            id: account_id,
            ext: ext.clone(),
        };

        // Both the keygen and signing will share a keystore
        let ecdsa_keystore = ECDSAKeyStore::in_memory(identity_pair);
        logger.trace("Starting protocol");
        let task = async move {
            if let Err(err) = crate::run::<_, MockBackend, _, _, _, _, _>(
                protocol_config,
                mock_client_keygen,
                mock_client_signing,
                logger.clone(),
                ecdsa_keystore,
                keygen_network,
                signing_network,
                pallet_tx,
            )
            .await
            {
                logger.error(format!("Error running test protocol: {err:?}"));
            }
        };

        tokio::task::spawn(task);
    }

    ext
}

fn mock_pub_key() -> ecdsa::Public {
    ecdsa_generate(KEY_TYPE, None)
}

pub mod mock_wrapper_client {
    use crate::mock::RuntimeOrigin;
    use async_trait::async_trait;
    use futures::StreamExt;
    use gadget_common::client::{AccountId, PalletSubmitter};
    use gadget_common::locks::TokioMutexExt;
    use gadget_common::Header;
    use gadget_core::gadget::substrate::Client;
    use sc_client_api::{
        BlockchainEvents, FinalityNotification, FinalityNotifications, ImportNotifications,
        StorageEventStream, StorageKey,
    };
    use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedSender};
    use sp_api::{ApiRef, BlockT, ProvideRuntimeApi};
    use sp_runtime::traits::Block;
    use std::sync::Arc;
    use std::time::Duration;
    use tangle_primitives::jobs::{JobId, JobResult};
    use tangle_primitives::roles::RoleType;
    use test_utils::sync::substrate_test_channel::MultiThreadedTestExternalities;

    #[derive(Clone)]
    pub struct MockClient<R, B: Block> {
        runtime: Arc<R>,
        finality_notification_stream: Arc<tokio::sync::Mutex<Option<FinalityNotifications<B>>>>,
        latest_finality_notification: Arc<tokio::sync::Mutex<Option<FinalityNotification<B>>>>,
        finality_notification_txs:
            Arc<parking_lot::Mutex<Vec<TracingUnboundedSender<FinalityNotification<B>>>>>,
    }

    impl<R, B: Block> MockClient<R, B> {
        pub async fn new(
            runtime: R,
            finality_notification_txs: Arc<
                parking_lot::Mutex<Vec<TracingUnboundedSender<FinalityNotification<B>>>>,
            >,
        ) -> Self {
            let runtime = Arc::new(runtime);
            let finality_notification_stream = Arc::new(tokio::sync::Mutex::new(None));

            let this = Self {
                runtime,
                finality_notification_stream,
                latest_finality_notification: tokio::sync::Mutex::new(None).into(),
                finality_notification_txs,
            };

            *this
                .finality_notification_stream
                .lock_timeout(Duration::from_millis(500))
                .await = Some(this.finality_notification_stream());
            this
        }
    }

    #[async_trait]
    impl<R: Send + Sync, B: Block> Client<B> for MockClient<R, B> {
        async fn get_next_finality_notification(&self) -> Option<FinalityNotification<B>> {
            let mut lock = self
                .finality_notification_stream
                .lock_timeout(Duration::from_millis(500))
                .await;
            let next = lock.as_mut().expect("Should exist").next().await;
            log::trace!(target: "gadget", "Latest Finality Notification: {:?}", next.as_ref().map(|r| r.header.number()));
            *self
                .latest_finality_notification
                .lock_timeout(Duration::from_millis(500))
                .await = next.clone();
            next
        }

        async fn get_latest_finality_notification(&self) -> Option<FinalityNotification<B>> {
            let lock = self
                .latest_finality_notification
                .lock_timeout(Duration::from_millis(500))
                .await;
            if let Some(latest) = lock.clone() {
                Some(latest)
            } else {
                drop(lock);
                self.get_next_finality_notification().await
            }
        }
    }

    impl<R: ProvideRuntimeApi<B>, B: Block> ProvideRuntimeApi<B> for MockClient<R, B> {
        type Api = R::Api;
        fn runtime_api(&self) -> ApiRef<Self::Api> {
            self.runtime.runtime_api()
        }
    }

    impl<R, B: Block> BlockchainEvents<B> for MockClient<R, B> {
        fn import_notification_stream(&self) -> ImportNotifications<B> {
            let (sink, stream) = tracing_unbounded("import_notification_stream", 1024);
            // We are not interested in block import notifications for tests
            std::mem::forget(sink);
            stream
        }

        fn every_import_notification_stream(&self) -> ImportNotifications<B> {
            unimplemented!()
        }

        fn finality_notification_stream(&self) -> FinalityNotifications<B> {
            let (sink, stream) =
                tracing_unbounded::<FinalityNotification<B>>("finality_notification_stream", 1024);
            self.finality_notification_txs.lock().push(sink);
            stream
        }

        fn storage_changes_notification_stream(
            &self,
            _filter_keys: Option<&[StorageKey]>,
            _child_filter_keys: Option<&[(StorageKey, Option<Vec<StorageKey>>)]>,
        ) -> sc_client_api::blockchain::Result<StorageEventStream<<B as BlockT>::Hash>> {
            unimplemented!()
        }
    }

    pub struct TestExternalitiesPalletSubmitter {
        pub ext: MultiThreadedTestExternalities,
        pub id: AccountId,
    }

    #[async_trait]
    impl PalletSubmitter for TestExternalitiesPalletSubmitter {
        async fn submit_job_result(
            &self,
            role_type: RoleType,
            job_id: JobId,
            result: JobResult,
        ) -> Result<(), gadget_common::Error> {
            let id = self.id;
            self.ext
                .execute_with_async(move || {
                    let origin = RuntimeOrigin::signed(id);
                    if let Err(err) =
                        crate::mock::Jobs::submit_job_result(origin, role_type, job_id, result)
                    {
                        let err = format!("Pallet tx error: {err:?}");
                        if err.contains("JobNotFound") {
                            // Job has already been submitted (assumption only for tests)
                            Ok(())
                        } else {
                            Err(gadget_common::Error::ClientError { err })
                        }
                    } else {
                        Ok(())
                    }
                })
                .await
        }
    }
}
