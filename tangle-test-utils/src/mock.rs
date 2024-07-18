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

use crate::mock::mock_wrapper_client::{MockClient, TestExternalitiesPalletSubmitter};
use crate::sync::substrate_test_channel::MultiThreadedTestExternalities;
use async_trait::async_trait;
use environment_utils::transaction_manager::tangle::TanglePalletSubmitter;
use frame_support::pallet_prelude::*;
use frame_support::traits::{Contains, Hooks, OneSessionHandler};
use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Everything},
    PalletId,
};
use frame_system::EnsureSigned;
use gadget_common::config::Network;
use gadget_common::locks::TokioMutexExt;
use gadget_common::prelude::{
    DebugLogger, ECDSAKeyStore, EventMetadata, GadgetEnvironment, InMemoryBackend, NodeInput,
    PrometheusConfig, UnboundedReceiver, UnboundedSender,
};
use gadget_common::tangle_subxt::subxt::OfflineClient;
use gadget_common::utils::serialize;
use gadget_common::Error;
use gadget_core::job_manager::{ProtocolMessageMetadata, SendFuture, WorkManagerInterface};
use pallet_services::EvmGasWeightMapping;
use sc_client_api::{FinalityNotification, FinalizeSummary};
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use serde::{Deserialize, Serialize};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_application_crypto::ecdsa;
use sp_core::{sr25519, ByteArray, Pair, H256};
use sp_keystore::testing::MemoryKeystore;
use sp_keystore::{KeystoreExt, KeystorePtr};
use sp_runtime::testing::UintAuthorityId;
use sp_runtime::traits::ConvertInto;
use sp_runtime::RuntimeDebug;
use sp_runtime::{traits::Block as BlockT, traits::IdentityLookup, BuildStorage, DispatchResult};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tangle_environment::gadget::{TangleEvent, TangleJobMetadata};
use tangle_environment::message::{TangleProtocolMessage, UserID};
use tangle_environment::work_manager::TangleWorkManager;
use tangle_primitives::AccountId;

pub type Balance = u128;
pub type BlockNumber = u64;

impl frame_system::Config for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
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
    type RuntimeTask = ();
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
    type RuntimeFreezeReason = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
}

parameter_types! {
    pub ElectionBoundsOnChain: ElectionBounds = ElectionBoundsBuilder::default()
        .voters_count(5_000.into()).targets_count(1_250.into()).build();
    pub ElectionBoundsMultiPhase: ElectionBounds = ElectionBoundsBuilder::default()
        .voters_count(10_000.into()).targets_count(1_500.into()).build();
}

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = AccountId;
    type FullIdentificationOf = ConvertInto;
}

pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
    fn contains(call: &RuntimeCall) -> bool {
        let is_stake_unbond_call = matches!(
            call,
            RuntimeCall::Staking(pallet_staking::Call::unbond { .. })
        );

        if is_stake_unbond_call {
            // no unbond call
            return false;
        }

        // no chill call
        if matches!(
            call,
            RuntimeCall::Staking(pallet_staking::Call::chill { .. })
        ) {
            return false;
        }

        // no withdraw_unbonded call
        let is_stake_withdraw_call = matches!(
            call,
            RuntimeCall::Staking(pallet_staking::Call::withdraw_unbonded { .. })
        );

        if is_stake_withdraw_call {
            return false;
        }

        true
    }
}

sp_runtime::impl_opaque_keys! {
    pub struct MockSessionKeys {
        pub other: MockSessionHandler,
    }
}

pub struct MockSessionHandler;
impl OneSessionHandler<AccountId> for MockSessionHandler {
    type Key = UintAuthorityId;

    fn on_genesis_session<'a, I: 'a>(_: I)
    where
        I: Iterator<Item = (&'a AccountId, Self::Key)>,
        AccountId: 'a,
    {
    }

    fn on_new_session<'a, I: 'a>(_: bool, _: I, _: I)
    where
        I: Iterator<Item = (&'a AccountId, Self::Key)>,
        AccountId: 'a,
    {
    }

    fn on_disabled(_validator_index: u32) {}
}

impl sp_runtime::BoundToRuntimeAppPublic for MockSessionHandler {
    type Public = UintAuthorityId;
}

pub struct MockSessionManager;

impl pallet_session::SessionManager<AccountId> for MockSessionManager {
    fn end_session(_: sp_staking::SessionIndex) {}
    fn start_session(_: sp_staking::SessionIndex) {}
    fn new_session(idx: sp_staking::SessionIndex) -> Option<Vec<AccountId>> {
        if idx == 0 || idx == 1 || idx == 2 {
            Some(vec![
                mock_pub_key(1),
                mock_pub_key(2),
                mock_pub_key(3),
                mock_pub_key(4),
            ])
        } else {
            None
        }
    }
}

parameter_types! {
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
}

impl pallet_session::Config for Runtime {
    type SessionManager = MockSessionManager;
    type Keys = MockSessionKeys;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionHandler = (MockSessionHandler,);
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ValidatorIdOf = pallet_staking::StashOf<Runtime>;
    type WeightInfo = ();
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
    type System = Runtime;
    type Solver = SequentialPhragmen<AccountId, Perbill>;
    type DataProvider = Staking;
    type WeightInfo = ();
    type MaxWinners = ConstU32<100>;
    type Bounds = ElectionBoundsOnChain;
}

/// Upper limit on the number of NPOS nominations.
const MAX_QUOTA_NOMINATIONS: u32 = 16;

impl pallet_staking::Config for Runtime {
    type Currency = Balances;
    type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
    type UnixTime = pallet_timestamp::Pallet<Self>;
    type CurrencyToVote = ();
    type RewardRemainder = ();
    type RuntimeEvent = RuntimeEvent;
    type Slash = ();
    type Reward = ();
    type SessionsPerEra = ();
    type SlashDeferDuration = ();
    type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type BondingDuration = ();
    type SessionInterface = ();
    type EraPayout = ();
    type NextNewSession = Session;
    type MaxExposurePageSize = ConstU32<64>;
    type MaxControllersInDeprecationBatch = ConstU32<100>;
    type OffendingValidatorsThreshold = ();
    type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type GenesisElectionProvider = Self::ElectionProvider;
    type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
    type TargetList = pallet_staking::UseValidatorsMap<Self>;
    type MaxUnlockingChunks = ConstU32<32>;
    type HistoryDepth = ConstU32<84>;
    type EventListeners = ();
    type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
    type NominationsQuota = pallet_staking::FixedNominationsQuota<MAX_QUOTA_NOMINATIONS>;
    type WeightInfo = ();
}

parameter_types! {
    pub const ServicesPalletId: PalletId = PalletId(*b"py/servs");
}

pub struct PalletEVMGasWeightMapping;

impl EvmGasWeightMapping for PalletEVMGasWeightMapping {
    fn gas_to_weight(gas: u64, without_base_weight: bool) -> Weight {
        pallet_evm::FixedGasWeightMapping::<Runtime>::gas_to_weight(gas, without_base_weight)
    }

    fn weight_to_gas(weight: Weight) -> u64 {
        pallet_evm::FixedGasWeightMapping::<Runtime>::weight_to_gas(weight)
    }
}

parameter_types! {
    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxFields: u32 = 256;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxFieldsSize: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxMetadataLength: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxJobsPerService: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxOperatorsPerService: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxPermittedCallers: u32 = 256;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxServicesPerOperator: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxBlueprintsPerOperator: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxServicesPerUser: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxBinariesPerGadget: u32 = 64;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxSourcesPerGadget: u32 = 64;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxGitOwnerLength: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxGitRepoLength: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxGitTagLength: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxBinaryNameLength: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxIpfsHashLength: u32 = 46;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxContainerRegistryLength: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxContainerImageNameLength: u32 = 1024;

    #[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub const MaxContainerImageTagLength: u32 = 1024;
}

impl pallet_services::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type Currency = Balances;
    type PalletId = ServicesPalletId;
    type EvmRunner = MockedEvmRunner;
    type EvmGasWeightMapping = PalletEVMGasWeightMapping;
    type MaxFields = MaxFields;
    type MaxFieldsSize = MaxFieldsSize;
    type MaxMetadataLength = MaxMetadataLength;
    type MaxJobsPerService = MaxJobsPerService;
    type MaxOperatorsPerService = MaxOperatorsPerService;
    type MaxPermittedCallers = MaxPermittedCallers;
    type MaxServicesPerOperator = MaxServicesPerOperator;
    type MaxBlueprintsPerOperator = MaxBlueprintsPerOperator;
    type MaxServicesPerUser = MaxServicesPerUser;
    type MaxBinariesPerGadget = MaxBinariesPerGadget;
    type MaxSourcesPerGadget = MaxSourcesPerGadget;
    type MaxGitOwnerLength = MaxGitOwnerLength;
    type MaxGitRepoLength = MaxGitRepoLength;
    type MaxGitTagLength = MaxGitTagLength;
    type MaxBinaryNameLength = MaxBinaryNameLength;
    type MaxIpfsHashLength = MaxIpfsHashLength;
    type MaxContainerRegistryLength = MaxContainerRegistryLength;
    type MaxContainerImageNameLength = MaxContainerImageNameLength;
    type MaxContainerImageTagLength = MaxContainerImageTagLength;
    type Constraints = pallet_services::types::ConstraintsOf<Self>;
    type WeightInfo = ();
}

type Block = frame_system::mocking::MockBlock<Runtime>;

#[allow(non_camel_case_types)]
construct_runtime!(
    pub enum Runtime
    {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        Services: pallet_services,
        Session: pallet_session,
        Staking: pallet_staking,
    }
);

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

pub fn id_to_ecdsa_pair(id: u8) -> ecdsa::Pair {
    ecdsa::Pair::from_string(&format!("//Alice///{id}"), None).expect("static values are valid")
}

pub fn id_to_sr25519_pair(id: u8) -> sr25519::Pair {
    sr25519::Pair::from_string(&format!("//Alice///{id}"), None).expect("static values are valid")
}

pub fn id_to_public(id: u8) -> ecdsa::Public {
    id_to_ecdsa_pair(id).public()
}

pub fn id_to_sr25519_public(id: u8) -> sr25519::Public {
    id_to_sr25519_pair(id).public()
}

sp_externalities::decl_extension! {
    pub struct TracingUnboundedReceiverExt(TracingUnboundedReceiver<<Block as BlockT>::Hash>);
}

type PeersRx<Env> =
    Arc<HashMap<ecdsa::Public, gadget_io::tokio::sync::Mutex<UnboundedReceiver<Env>>>>;

pub struct MockNetwork<Env: GadgetEnvironment> {
    peers_tx: Arc<
        HashMap<
            ecdsa::Public,
            UnboundedSender<<Env::WorkManager as WorkManagerInterface>::ProtocolMessage>,
        >,
    >,
    peers_rx: PeersRx<<Env::WorkManager as WorkManagerInterface>::ProtocolMessage>,
    my_id: ecdsa::Public,
}

impl<Env: GadgetEnvironment> Clone for MockNetwork<Env> {
    fn clone(&self) -> Self {
        Self {
            peers_tx: self.peers_tx.clone(),
            peers_rx: self.peers_rx.clone(),
            my_id: self.my_id,
        }
    }
}

impl<Env: GadgetEnvironment> MockNetwork<Env> {
    pub fn setup(ids: &Vec<ecdsa::Public>) -> Vec<Self> {
        let mut peers_tx = HashMap::new();
        let mut peers_rx = HashMap::new();
        let mut networks = Vec::new();

        for id in ids {
            let (tx, rx) = gadget_io::tokio::sync::mpsc::unbounded_channel();
            peers_tx.insert(*id, tx);
            peers_rx.insert(*id, gadget_io::tokio::sync::Mutex::new(rx));
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
impl<Env: GadgetEnvironment> Network<Env> for MockNetwork<Env>
where
    Env::ProtocolMessage: Clone,
{
    async fn next_message(
        &self,
    ) -> Option<<Env::WorkManager as WorkManagerInterface>::ProtocolMessage> {
        self.peers_rx
            .get(&self.my_id)?
            .lock_timeout(Duration::from_millis(500))
            .await
            .recv()
            .await
    }

    async fn send_message(
        &self,
        message: <Env::WorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), Error> {
        let _check_message_has_ids = message.sender_network_id().ok_or(Error::MissingNetworkId)?;
        if let Some(peer_id) = message.recipient_network_id() {
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
        Services::on_finalize(System::block_number());
        Balances::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        Services::on_initialize(System::block_number());
        Balances::on_initialize(System::block_number());
    }
}

// Checks events against the latest. A contiguous set of events must be
// provided. They must include the most recent RuntimeEvent, but do not have to include
// every past RuntimeEvent.
pub fn assert_events(mut expected: Vec<RuntimeEvent>) {
    let mut actual: Vec<RuntimeEvent> = System::events().iter().map(|e| e.event.clone()).collect();

    expected.reverse();
    for evt in expected {
        let next = actual.pop().expect("RuntimeEvent expected");
        assert_eq!(next, evt, "Events don't match (actual,expected)");
    }
}

/// This function basically just builds a genesis storage key/value store according to
/// our desired mockup.
/// N: number of nodes
/// K: Number of networks accessible per node
/// D: Any data that you want to pass to pass with NodeInput.
/// F: A function that generates a singular full node (all possible protocols) by returning a future representing the node's execution
pub async fn new_test_ext<
    const N: usize,
    const K: usize,
    D: Send + Clone + 'static,
    F: Fn(
        NodeInput<TangleExtEnvironment, MockNetwork<TangleExtEnvironment>, InMemoryBackend, D>,
    ) -> Fut,
    Fut: SendFuture<'static, ()>,
>(
    additional_params: D,
    f: F,
) -> MultiThreadedTestExternalities {
    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    let role_pairs = (0..N)
        .map(|i| id_to_ecdsa_pair(i as u8))
        .collect::<Vec<_>>();
    let roles_identities = role_pairs
        .iter()
        .map(|pair| pair.public())
        .collect::<Vec<_>>();

    let pairs = (0..N)
        .map(|i| id_to_sr25519_pair(i as u8))
        .collect::<Vec<_>>();
    let account_ids = pairs
        .iter()
        .map(|pair| pair.public().into())
        .collect::<Vec<AccountId>>();

    let balances = account_ids
        .iter()
        .map(|public| (public.clone(), 100u128))
        .collect::<Vec<_>>();

    let networks = (0..K)
        .map(|_| MockNetwork::setup(&roles_identities))
        .collect::<Vec<_>>();

    // Transpose networks
    let networks = (0..N)
        .map(|i| {
            networks
                .iter()
                .map(|network| network[i].clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    pallet_balances::GenesisConfig::<Runtime> { balances }
        .assimilate_storage(&mut t)
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.register_extension(KeystoreExt(Arc::new(MemoryKeystore::new()) as KeystorePtr));

    let ext = MultiThreadedTestExternalities::new(ext);
    assert!(TEST_EXTERNALITIES.lock().replace(ext.clone()).is_none(), "Make sure to run tests serially with -- --test-threads=1 or with nextest to ensure separate program spaces per test");

    let finality_notification_txs = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let sinks = finality_notification_txs.clone();
    let externalities = ext.clone();

    // Spawn a thread that sends a finality notification whenever it detects a change in block number
    gadget_io::tokio::task::spawn(async move {
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
                    let notification = generate_finality_notification_at(number);
                    if sink.unbounded_send(notification).is_err() {
                        log::warn!(target: "gadget", "Will not deliver FinalityNotification because the receiver is gone");
                    }
                }
            }

            gadget_io::tokio::time::sleep(Duration::from_millis(12000)).await;
        }
    });

    for (node_index, ((role_pair, pair), networks)) in
        role_pairs.into_iter().zip(pairs).zip(networks).enumerate()
    {
        let mut mock_clients = Vec::new();

        for _ in 0..K {
            mock_clients.push(MockClient::new(Runtime, finality_notification_txs.clone()).await);
        }

        let account_id: AccountId = pair.public().into();

        let logger = DebugLogger {
            id: format!("Peer {node_index}"),
        };

        let genesis_hash = generate_finality_notification_at(0).hash;
        let tx_manager = Arc::new(TestExternalitiesPalletSubmitter {
            id: account_id.clone(),
            ext: ext.clone(),
            subxt_client: OfflineClient::new(genesis_hash, "1.0.0".into(), []),
        });

        let keystore = ECDSAKeyStore::in_memory(role_pair);
        let prometheus_config = PrometheusConfig::Disabled;

        let input = NodeInput {
            clients: mock_clients,
            networks,
            account_id: sr25519::Public(account_id.into()),
            logger,
            tx_manager: tx_manager as _,
            keystore,
            node_index,
            additional_params: additional_params.clone(),
            prometheus_config,
        };

        let task = f(input);
        gadget_io::tokio::task::spawn(task);
    }

    ext
}

fn generate_finality_notification_at(number: u64) -> FinalityNotification<Block> {
    let (faux_sink, faux_stream) = tracing_unbounded("faux_sink", 1024);
    std::mem::forget(faux_stream);
    let header = <Block as BlockT>::Header::new_from_number(number);
    let summary = FinalizeSummary::<Block> {
        finalized: vec![header.hash()],
        header,
        stale_heads: vec![],
    };

    FinalityNotification::from_summary(summary, faux_sink)
}

pub mod mock_wrapper_client {
    use crate::mock::{Runtime, TangleExtEnvironment};
    use crate::sync::substrate_test_channel::MultiThreadedTestExternalities;
    use async_trait::async_trait;
    use environment_utils::transaction_manager::tangle::TanglePalletSubmitter;
    use futures::StreamExt;
    use gadget_common::client::exec_client_function;
    use gadget_common::locks::TokioMutexExt;
    use gadget_common::tangle_runtime::*;
    use gadget_common::tangle_subxt;
    use gadget_common::tangle_subxt::subxt::utils::AccountId32;
    use gadget_common::tangle_subxt::subxt::{Config, OfflineClient, OnlineClient};
    use gadget_core::gadget::general::Client;
    use gadget_core::gadget::substrate;
    use gadget_io::tokio;
    use parity_scale_codec::{Decode, Encode};
    use sc_client_api::{
        BlockchainEvents, FinalityNotification, FinalityNotifications, ImportNotifications,
        StorageEventStream, StorageKey,
    };
    use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedSender};
    use sp_api::{ApiRef, ProvideRuntimeApi};
    use sp_runtime::traits::Block;
    use sp_runtime::traits::Header;
    use std::sync::Arc;
    use std::time::Duration;
    use tangle_environment::gadget::TangleEvent;
    use tangle_primitives::AccountId;

    #[derive(Clone)]
    pub struct MockClient<R, B: Block> {
        runtime: Arc<R>,
        finality_notification_stream:
            Arc<gadget_io::tokio::sync::Mutex<Option<FinalityNotifications<B>>>>,
        latest_finality_notification: Arc<gadget_io::tokio::sync::Mutex<Option<TangleEvent>>>,
        finality_notification_txs:
            Arc<parking_lot::Mutex<Vec<TracingUnboundedSender<TangleEvent>>>>,
    }

    impl<R, B: Block> MockClient<R, B> {
        pub async fn new(
            runtime: R,
            finality_notification_txs: Arc<
                parking_lot::Mutex<Vec<TracingUnboundedSender<TangleEvent>>>,
            >,
        ) -> Self {
            let runtime = Arc::new(runtime);
            let finality_notification_stream = Arc::new(gadget_io::tokio::sync::Mutex::new(None));

            let this = Self {
                runtime,
                finality_notification_stream,
                latest_finality_notification: gadget_io::tokio::sync::Mutex::new(None).into(),
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
    impl<R: Send + Sync + Clone, B: Block> Client<TangleEvent> for MockClient<R, B> {
        async fn next_event(&self) -> Option<TangleEvent> {
            let mut lock = self
                .finality_notification_stream
                .lock_timeout(Duration::from_millis(500))
                .await;
            let next = lock.as_mut().expect("Should exist").next().await;
            log::trace!(target: "gadget", "Latest Finality Notification: {:?}", next.as_ref().map(|r| r.header.number()));
            self.latest_finality_notification
                .lock_timeout(Duration::from_millis(500))
                .await
                .clone_from(&next);

            next.map(|n| {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(n.header.hash().as_ref());
                let number =
                    Decode::decode(&mut Encode::encode(&n.header.number()).as_slice()).unwrap();
                TangleEvent {
                    hash,
                    number,
                    events: n,
                }
            })
        }

        async fn latest_event(&self) -> Option<TangleEvent> {
            let lock = self
                .latest_finality_notification
                .lock_timeout(Duration::from_millis(500))
                .await;
            if let Some(n) = lock.clone() {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&n.hash);
                let number = Decode::decode(&mut Encode::encode(&n.number).as_slice()).unwrap();
                Some(TangleEvent {
                    hash,
                    number,
                    events: n,
                })
            } else {
                drop(lock);
                self.next_event().await
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
        ) -> sc_client_api::blockchain::Result<StorageEventStream<<B as Block>::Hash>> {
            unimplemented!()
        }
    }

    pub struct TestExternalitiesPalletSubmitter<C: Config> {
        pub ext: MultiThreadedTestExternalities,
        pub id: AccountId,
        pub subxt_client: OfflineClient<C>,
    }

    impl<C: Config> std::fmt::Debug for TestExternalitiesPalletSubmitter<C> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TestExternalitiesPalletSubmitter")
                .field("id", &self.id.to_string())
                .finish()
        }
    }

    #[async_trait]
    impl<C: Config> TanglePalletSubmitter for TestExternalitiesPalletSubmitter<C> {
        async fn submit_service_result(
            &self,
            service_id: u64,
            call_id: u64,
            result: api::services::calls::types::submit_result::Result,
        ) -> Result<(), gadget_common::Error> {
            let id = self.id.clone();
            let client = self.subxt_client.clone();
            let function = async move {
                let call = api::tx()
                    .services()
                    .submit_result(service_id, call_id, result);
                let _res = client
                    .tx()
                    .sign_and_submit_then_watch_default(&call, &id)
                    .await?
                    .wait_for_finalized_success()
                    .await?
                    .block_hash();

                Ok(())
            };

            function.await
        }
    }
}

#[derive(Clone, Debug)]
pub struct TangleExtEnvironment {
    finality_notification_txs:
        Arc<parking_lot::Mutex<Vec<TracingUnboundedSender<FinalityNotification<Block>>>>>,
    #[allow(dead_code)]
    tx_manager: Arc<parking_lot::Mutex<Option<Arc<dyn TanglePalletSubmitter>>>>,
}

#[async_trait]
impl GadgetEnvironment for TangleExtEnvironment {
    type Event = TangleEvent;
    type ProtocolMessage = TangleProtocolMessage;
    type Client = MockClient<Runtime, Block>;
    type WorkManager = TangleWorkManager;
    type Error = gadget_common::Error;
    type Clock = <Self::WorkManager as WorkManagerInterface>::Clock;
    type RetryID = <Self::WorkManager as WorkManagerInterface>::RetryID;
    type TaskID = <Self::WorkManager as WorkManagerInterface>::TaskID;
    type SessionID = <Self::WorkManager as WorkManagerInterface>::SessionID;
    type TransactionManager = Arc<dyn TanglePalletSubmitter>;
    type JobInitMetadata = TangleJobMetadata;

    fn build_protocol_message<Payload: Serialize>(
        associated_block_id: <Self::WorkManager as WorkManagerInterface>::Clock,
        associated_session_id: <Self::WorkManager as WorkManagerInterface>::SessionID,
        associated_retry_id: <Self::WorkManager as WorkManagerInterface>::RetryID,
        associated_task_id: <Self::WorkManager as WorkManagerInterface>::TaskID,
        from: UserID,
        to: Option<UserID>,
        payload: &Payload,
        from_account_id: Option<ecdsa::Public>,
        to_network_id: Option<ecdsa::Public>,
    ) -> Self::ProtocolMessage {
        TangleProtocolMessage {
            associated_block_id,
            associated_session_id,
            associated_retry_id,
            task_hash: associated_task_id,
            from,
            to,
            payload: serialize(payload).expect("Failed to serialize message"),
            from_network_id: from_account_id,
            to_network_id,
        }
    }

    async fn setup_runtime(&self) -> Result<Self::Client, Self::Error> {
        let finality_notification_txs = self.finality_notification_txs.clone();
        Ok(MockClient::<Runtime, Block>::new(Runtime, finality_notification_txs).await)
    }

    fn transaction_manager(&self) -> Self::TransactionManager {
        if let Some(tx_manager) = &*self.tx_manager.lock() {
            tx_manager.clone()
        } else {
            panic!("Transaction manager not set")
        }
    }
}

impl EventMetadata<TangleExtEnvironment> for TangleEvent {
    fn number(&self) -> <TangleExtEnvironment as GadgetEnvironment>::Clock {
        self.number
    }
}

pub fn mock_pub_key(id: u8) -> AccountId {
    sr25519::Public::from_raw([id; 32]).into()
}
