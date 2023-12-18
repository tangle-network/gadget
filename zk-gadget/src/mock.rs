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

use frame_support::{
    construct_runtime,
    pallet_prelude::*,
    parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Everything},
    PalletId,
};
use frame_system::EnsureSigned;
use gadget_core::gadget::substrate::Client;
use sc_client_api::{BlockImportNotification, FinalityNotification, FinalizeSummary};
use sc_utils::mpsc::{TracingUnboundedReceiver, TracingUnboundedSender};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_application_crypto::RuntimePublic;
use sp_core::H256;
use sp_runtime::create_runtime_str;
use sp_runtime::{
    traits::Block as BlockT, traits::IdentityLookup, AccountId32, BuildStorage, DispatchResult,
};
use sp_version::RuntimeVersion;
pub type Balance = u128;
pub type BlockNumber = u64;

use sp_core::ecdsa;
use sp_io::crypto::ecdsa_generate;
use sp_keystore::{testing::MemoryKeystore, KeystoreExt, KeystorePtr};
use sp_std::sync::Arc;
use tangle_primitives::{
    jobs::{
        DkgKeyType, JobId, JobKey, JobSubmission, JobType, JobWithResult, ReportValidatorOffence,
        RpcResponseJobsData, RpcResponsePhaseOneResult, ValidatorOffenceType,
    },
    roles::{RoleTypeMetadata, TssRoleMetadata},
    traits::{
        jobs::{JobToFee, MPCHandler},
        roles::RolesHandler,
    },
};

use crate::client_ext::ClientWithApi;

/// Key type for DKG keys
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"wdkg");

type Block = frame_system::mocking::MockBlock<Runtime>;

impl frame_system::Config for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId32;
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
    fn job_to_fee(job: &JobSubmission<AccountId32, BlockNumber>) -> Balance {
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
    fn job_to_fee(job: &JobSubmission<AccountId32, BlockNumber>) -> Balance {
        if job.job_type.is_phase_one() {
            10
        } else {
            20
        }
    }
}

pub struct MockJobToFeeHandler;

impl JobToFee<AccountId32, BlockNumber> for MockJobToFeeHandler {
    type Balance = Balance;

    fn job_to_fee(job: &JobSubmission<AccountId32, BlockNumber>) -> Balance {
        match job.job_type {
            JobType::DKGTSSPhaseOne(_) => MockDKGPallet::job_to_fee(job),
            JobType::DKGTSSPhaseTwo(_) => MockDKGPallet::job_to_fee(job),
            JobType::ZkSaaSPhaseOne(_) => MockZkSaasPallet::job_to_fee(job),
            JobType::ZkSaaSPhaseTwo(_) => MockZkSaasPallet::job_to_fee(job),
        }
    }
}

pub struct MockRolesHandler;

impl RolesHandler<AccountId32> for MockRolesHandler {
    fn is_validator(address: AccountId32, _role_type: JobKey) -> bool {
        let validators = [
            AccountId32::new([1u8; 32]),
            AccountId32::new([2u8; 32]),
            AccountId32::new([3u8; 32]),
            AccountId32::new([4u8; 32]),
            AccountId32::new([5u8; 32]),
            AccountId32::new([6u8; 32]),
            AccountId32::new([7u8; 32]),
            AccountId32::new([8u8; 32]),
        ];
        validators.contains(&address)
    }

    fn report_offence(_offence_report: ReportValidatorOffence<AccountId32>) -> DispatchResult {
        Ok(())
    }

    fn get_validator_metadata(address: AccountId32, _job_key: JobKey) -> Option<RoleTypeMetadata> {
        let mock_err_account = AccountId32::new([100u8; 32]);
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

impl MPCHandler<AccountId32, BlockNumber, Balance> for MockMPCHandler {
    fn verify(_data: JobWithResult<AccountId32>) -> DispatchResult {
        Ok(())
    }

    fn verify_validator_report(
        _validator: AccountId32,
        _offence: ValidatorOffenceType,
        _signatures: Vec<Vec<u8>>,
    ) -> DispatchResult {
        Ok(())
    }

    fn validate_authority_key(_validator: AccountId32, _authority_key: Vec<u8>) -> DispatchResult {
        Ok(())
    }
}

parameter_types! {
    pub const JobsPalletId: PalletId = PalletId(*b"py/jobss");
}

impl pallet_jobs::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ForceOrigin = EnsureSigned<AccountId32>;
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

    async fn get_next_block_import_notification(&self) -> Option<BlockImportNotification<Block>> {
        None
    }
}

impl ProvideRuntimeApi<Block> for Runtime {
    type Api = Self;
    fn runtime_api(&self) -> ApiRef<Self::Api> {
        ApiRef::from(*self)
    }
}

sp_api::mock_impl_runtime_apis! {
    impl pallet_jobs_rpc_runtime_api::JobsApi<Block, AccountId32> for Runtime {
        fn query_jobs_by_validator(validator: AccountId32) -> Result<Vec<RpcResponseJobsData<AccountId32>>, String> { todo!() }
        fn query_job_by_id(job_key: JobKey, job_id: JobId) -> Option<RpcResponseJobsData<AccountId32>> { todo!() }
        fn query_phase_one_by_id(job_key: JobKey, job_id: JobId) -> Option<RpcResponsePhaseOneResult<AccountId32>> { todo!() }
        fn query_next_job_id() -> JobId { todo!() }
    }
}

pub struct ExtBuilder;

impl Default for ExtBuilder {
    fn default() -> Self {
        ExtBuilder
    }
}

pub fn to_account_id32(id: u8) -> AccountId32 {
    AccountId32::new([id; 32])
}

sp_externalities::decl_extension! {
    pub struct TracingUnboundedReceiverExt(TracingUnboundedReceiver<<Block as BlockT>::Hash>);
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: [
            (to_account_id32(1), 100u128),
            (to_account_id32(2), 100u128),
            (to_account_id32(3), 100u128),
            (to_account_id32(4), 100u128),
            (to_account_id32(5), 100u128),
            (to_account_id32(6), 100u128),
            (to_account_id32(7), 100u128),
            (to_account_id32(8), 100u128),
            (to_account_id32(9), 100u128),
            (to_account_id32(10), 100u128),
            (to_account_id32(20), 100u128),
        ]
        .to_vec(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    // set to block 1 to test events
    ext.execute_with(|| System::set_block_number(1));
    ext.register_extension(KeystoreExt(Arc::new(MemoryKeystore::new()) as KeystorePtr));

    let mock_client = Runtime;
    let config = crate::ZkGadgetConfig {
        king_bind_addr: todo!(),
        client_only_king_addr: todo!(),
        network_id: todo!(),
        public_identity_der: todo!(),
        private_identity_der: todo!(),
        client_only_king_public_identity_der: todo!(),
        account_id: todo!(),
    };
    crate::run(config, mock_client);
    ext
}

fn mock_pub_key() -> ecdsa::Public {
    ecdsa_generate(KEY_TYPE, None)
}
