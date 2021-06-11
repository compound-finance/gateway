use crate::{self as pallet_cash, *};
use codec::alloc::sync::Arc;
use parking_lot::RwLock;
use sp_core::{
    offchain::{
        testing::{self, OffchainState, PoolState},
        OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
    },
    H256,
};
use sp_runtime::{
    generic, impl_opaque_keys,
    testing::{Header, TestXt, UintAuthorityId},
    traits::{
        BlakeTwo256, ConvertInto, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup,
        OpaqueKeys, Verify,
    },
    MultiAddress, MultiSignature as Signature,
};

pub type Extrinsic = TestXt<Call, ()>;
pub type CashModule = Module<Test>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

pub const MILLISECS_PER_BLOCK: types::Timestamp = 6000;
pub const SLOT_DURATION: types::Timestamp = MILLISECS_PER_BLOCK;

pub type Address = MultiAddress<AccountId, ()>;
pub type SignedExtra = (
    frame_system::CheckSpecVersion<Test>,
    frame_system::CheckTxVersion<Test>,
    frame_system::CheckGenesis<Test>,
    frame_system::CheckEra<Test>,
    frame_system::CheckNonce<Test>,
    frame_system::CheckWeight<Test>,
);
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

// XXX just to compile, these should maybe use the real impls
pub struct TestShouldEndSession;
impl pallet_session::ShouldEndSession<u64> for TestShouldEndSession {
    fn should_end_session(_now: u64) -> bool {
        true
    }
}
use sp_runtime::RuntimeAppPublic;

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<SubstrateId> for TestSessionHandler {
    const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[UintAuthorityId::ID];
    fn on_genesis_session<T: OpaqueKeys>(_validators: &[(SubstrateId, T)]) {}
    fn on_new_session<T: OpaqueKeys>(
        _changed: bool,
        _validators: &[(SubstrateId, T)],
        _queued_validators: &[(SubstrateId, T)],
    ) {
    }
    fn on_disabled(_validator_index: usize) {}
    fn on_before_session_ending() {}
}

pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    impl_opaque_keys! {
        pub struct MockSessionKeys {
            pub dummy: UintAuthorityId,
        }
    }
    impl From<UintAuthorityId> for MockSessionKeys {
        fn from(dummy: UintAuthorityId) -> Self {
            Self { dummy }
        }
    }
}

frame_support::construct_runtime!(
    pub enum Test where
    Block = Block,
    NodeBlock = Block,
    UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Oracle: pallet_oracle::{Pallet, Call, Config, Storage, Event, Inherent},
        Cash: pallet_cash::{Pallet, Call, Config, Storage, Event, Inherent},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
    }
);

frame_support::parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const MinimumPeriod: types::Timestamp = SLOT_DURATION / 2;
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type OnSetCode = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

impl pallet_session::Config for Test {
    type Event = pallet_session::Event;
    type ValidatorId = SubstrateId;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = TestShouldEndSession;
    type NextSessionRotation = ();
    type SessionManager = Cash;
    type SessionHandler = TestSessionHandler;
    type Keys = opaque::MockSessionKeys;
    type DisabledValidatorsThreshold = (); //sp_runtime::Perbill::from_percent(33);
    type WeightInfo = ();
}

impl pallet_timestamp::Config for Test {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = types::Timestamp;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl pallet_oracle::Config for Test {
    type Call = Call;
    type Event = Event;
}

impl frame_system::offchain::SigningTypes for Test {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

impl Config for Test {
    type Event = Event;
    type Call = Call;
    type TimeConverter = crate::converters::TimeConverter<Self>;
    type AccountStore = System;
    type SessionInterface = Self;
    type WeightInfo = ();
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
    Call: From<LocalCall>,
{
    type OverarchingCall = Call;
    type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
    Call: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        _public: <Signature as Verify>::Signer,
        _account: AccountId,
        nonce: u64,
    ) -> Option<(Call, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
        Some((call, (nonce, ())))
    }
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let (t, _pool_state, _offchain_state) = new_test_ext_with_http_calls(vec![]);
    t
}

pub fn new_test_ext_with_http_calls(
    mut calls: Vec<testing::PendingRequest>,
) -> (
    sp_io::TestExternalities,
    Arc<RwLock<PoolState>>,
    Arc<RwLock<OffchainState>>,
) {
    let (offchain, offchain_state) = testing::TestOffchainExt::new();
    let (pool, pool_state) = testing::TestTransactionPoolExt::new();

    // XXX
    // let mut test_externalities: sp_io::TestExternalities = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();
    let mut test_externalities = sp_io::TestExternalities::default();
    test_externalities.register_extension(OffchainDbExt::new(offchain.clone()));
    test_externalities.register_extension(OffchainWorkerExt::new(offchain));
    test_externalities.register_extension(TransactionPoolExt::new(pool));

    {
        let mut state = offchain_state.write();
        for call in calls.drain(0..calls.len()) {
            state.expect_request(call);
        }
    }

    test_externalities.execute_with(|| System::set_block_number(1));
    (test_externalities, pool_state, offchain_state)
}
