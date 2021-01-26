use crate::{Call, Config, Module};
use frame_support::{assert_ok, impl_outer_event, impl_outer_origin, parameter_types};
use sp_core::{
    offchain::{testing, OffchainExt, TransactionPoolExt},
    sr25519::Signature,
    H256,
};
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
};

impl_outer_origin! {
    pub enum Origin for Test {}
}

// Configure a mock runtime to test the pallet.

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = sp_core::sr25519::Public;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = TestEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = ();
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

mod cash {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        cash<T>,
        frame_system<T>,
    }
}

type Extrinsic = TestXt<Call<Test>, ()>;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

impl frame_system::offchain::SigningTypes for Test {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

pub type System = frame_system::Module<Test>;

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
    Call<Test>: From<LocalCall>,
{
    type OverarchingCall = Call<Test>;
    type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
    Call<Test>: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call<Test>,
        _public: <Signature as Verify>::Signer,
        _account: AccountId,
        nonce: u64,
    ) -> Option<(Call<Test>, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
        Some((call, (nonce, ())))
    }
}

impl Config for Test {
    type Event = TestEvent;
    type Call = Call<Test>;
}

pub type CashModule = Module<Test>;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    new_test_ext_with_http_calls(vec![])
}

pub fn new_test_ext_with_http_calls(
    mut calls: Vec<testing::PendingRequest>,
) -> sp_io::TestExternalities {
    let (offchain, state) = testing::TestOffchainExt::new();
    let (pool, pool_state) = testing::TestTransactionPoolExt::new();

    // let mut test_externalities: sp_io::TestExternalities = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();

    let mut test_externalities = sp_io::TestExternalities::default();
    test_externalities.register_extension(OffchainExt::new(offchain));
    test_externalities.register_extension(TransactionPoolExt::new(pool));

    {
        let mut state = state.write();
        for call in calls.drain(0..calls.len()) {
            state.expect_request(call);
        }
    }

    test_externalities.execute_with(|| System::set_block_number(1));
    test_externalities
}
