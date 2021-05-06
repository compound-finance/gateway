#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::{
    chains::{Chain, ChainAsset, ChainSignatureList, Ethereum},
    notices::{ExtractionNotice, Notice},
    rates::APR,
    types::*,
    types::{AssetInfo, Factor, ValidatorKeys},
    Pallet as Cash,
};
use codec::EncodeLike;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
pub use frame_support::{
    assert_err, assert_ok,
    traits::{OnInitialize, OriginTrait},
    StorageValue,
};
use frame_system::RawOrigin;
use hex_literal::hex;
use num_traits::Zero;
use sp_core::crypto::AccountId32;
use sp_std::prelude::*;

pub use our_std::{convert::TryInto, str::FromStr};
use pallet_oracle::Prices;

const TKN_ADDR: &str = "0x0101010101010101010101010101010101010101";
const TKN_ADDR_BYTES: [u8; 20] = [1; 20];

const ETH_ADDR: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";
const ETH_BYTES: [u8; 20] = hex!("EeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE");

const ALICE_ADDRESS: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const BOB_ADDRESS: &str = "0x59a055a3e566F5d9A9Ea1dA81aB375D5361D7c5e";
const BOB_ADDRESS_BYTES: [u8; 20] = hex!("59a055a3e566F5d9A9Ea1dA81aB375D5361D7c5e");

const MIN_TX_VALUE: u128 = params::MIN_TX_VALUE.value;

// endow token to user, create market, add some dummy data
fn endow_tkn<T: Config>(
    holder: [u8; 20],
    amount: AssetBalance,
    addr: <Ethereum as Chain>::Address,
) {
    let asset = ChainAsset::Eth(addr);
    let asset_info = AssetInfo {
        liquidity_factor: LiquidityFactor::from_nominal("1"),
        miner_shares: MinerShares::from_nominal("0.02"),
        ..AssetInfo::minimal(asset, Units::from_ticker_str("TKN", 6))
    };

    SupportedAssets::insert(&asset, asset_info);
    Prices::insert(asset_info.ticker, 1_000_000); // $1

    SupplyIndices::insert(&asset, AssetIndex::from_nominal("1234"));
    BorrowIndices::insert(&asset, AssetIndex::from_nominal("1345"));
    init_asset_balance(asset, ChainAccount::Eth(holder), amount);
}

// todo: consolidate w tests::common
fn init_asset_balance(asset: ChainAsset, account: ChainAccount, balance: AssetBalance) {
    AssetBalances::insert(asset, account, balance);
    if balance >= 0 {
        TotalSupplyAssets::insert(
            asset,
            (TotalSupplyAssets::get(asset) as i128 + balance) as u128,
        );
    } else {
        TotalBorrowAssets::insert(
            asset,
            (TotalBorrowAssets::get(asset) as i128 + balance) as u128,
        );
    }
    AssetsWithNonZeroBalance::insert(account, asset, ());
}

benchmarks! {
  where_clause {
    where
    T: pallet_session::Config,
    T: pallet_timestamp::Config,
    u64: EncodeLike<<T as pallet_timestamp::Config>::Moment>,
    <<T as frame_system::Config>::Origin as OriginTrait>::AccountId: From<SubstrateId>
  }

  // test gas up to 10 tokens
  on_initialize {
    let z in 0 .. 10;
    let signer_vec = <Ethereum as Chain>::signer_address().unwrap();
    for i in 0..z {
      let i: u8 = z.try_into().unwrap();
      let addr_bytes: [u8; 20] = [i; 20];
      let extract_amt: i128 = MIN_TX_VALUE.try_into().unwrap();
      endow_tkn::<T>(signer_vec, extract_amt, addr_bytes);
    }
    <pallet_timestamp::Now<T>>::put(1u64);
    LastYieldTimestamp::put(1u64);
    LastBlockTimestamp::put(1u64);

    let miner = ChainAccount::Eth([0; 20]);
    GlobalCashIndex::put(CashIndex::from_nominal("1.123"));
    LastYieldCashIndex::put(CashIndex::from_nominal("1.123"));
    CashYield::put(APR::from_nominal("0.24"));
    TotalCashPrincipal::put(CashPrincipalAmount::from_nominal("450000")); // 450k cash principal
    CashPrincipals::insert(&miner, CashPrincipal::from_nominal("1"));
  }: {
    assert_ne!(Pallet::<T>::on_initialize(T::BlockNumber::zero()), 0);
  }

  publish_signature {
    let chain_id = ChainId::Eth;
    let notice_id = NoticeId(5, 6);
    let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
        id: NoticeId(80, 1),
        parent: [3u8; 32],
        asset: [1; 20],
        amount: 100,
        account: [2; 20],
    });
    let signature = notice.sign_notice().unwrap();
    let eth_signature = match signature {
        ChainSignature::Eth(a) => a,
        _ => panic!("absurd"),
    };
    let notice_state = NoticeState::Pending {
        signature_pairs: ChainSignatureList::Eth(vec![]),
    };
    NoticeStates::insert(chain_id, notice_id, notice_state);
    Notices::insert(chain_id, notice_id, notice);
    let substrate_id = AccountId32::new([0u8; 32]);
    let eth_address = <Ethereum as Chain>::signer_address().unwrap();
    Validators::insert(
        substrate_id.clone(),
        ValidatorKeys {
            substrate_id,
            eth_address,
        },
    );

    let expected_notice_state = NoticeState::Pending {
       signature_pairs: ChainSignatureList::Eth(vec![(eth_address, eth_signature)]),
    };

    }: {
        assert_eq!(Cash::<T>::publish_signature(RawOrigin::None.into(), chain_id, notice_id, signature), Ok(()));
    } verify {
        assert_eq!(
            NoticeStates::get(chain_id, notice_id),
            expected_notice_state
        );
    }

    set_yield_next {
        assert_eq!(CashYieldNext::get(), None);
    }: {
        <pallet_timestamp::Now<T>>::put(1u64);
        assert_eq!(Cash::<T>::set_yield_next(RawOrigin::Root.into(), APR(100).into(), 86400500), Ok(()));
    }

    // receive_chain_blocks {
    //   let substrate_id = AccountId32::new([12u8; 32]);
    //   let eth_address = <Ethereum as Chain>::signer_address().unwrap();
    //   Validators::insert(
    //       substrate_id.clone(),
    //       ValidatorKeys {
    //           substrate_id,
    //           eth_address,
    //       },
    //   );
    //   let dummy_last_block = ethereum_client::EthereumBlock {
    //     hash: [0;32],
    //     parent_hash: [0;32],
    //     number: 1u64,
    //     events: vec![]
    //   };
    //   LastProcessedBlock::insert(ChainId::Eth, dummy_last_block);
    //   let blocks = ChainBlocks::Eth(vec![]);
    //   let signature = ChainSignature::Eth(<Ethereum as Chain>::sign_message(&blocks.encode()).unwrap());
    // }: {
    //     assert_ok!(Cash::<T>::receive_chain_blocks(RawOrigin::None.into(), blocks, signature));
    // }

  support_asset {
    let info = AssetInfo::minimal(
      ChainAsset::Eth([1u8; 20]),
      FromStr::from_str("USDC/6").unwrap(),
    );
  }: {
    assert_ok!(Cash::<T>::support_asset(RawOrigin::Root.into(), info));
  }

  set_rate_model{
    let info = AssetInfo::minimal(
      ChainAsset::Eth([1u8; 20]),
      FromStr::from_str("USDC/6").unwrap(),
    );
    assert_ok!(Cash::<T>::support_asset(RawOrigin::Root.into(), info));
  }: {
    assert_ok!(Cash::<T>::set_rate_model(RawOrigin::Root.into(), ChainAsset::Eth([1u8; 20]), InterestRateModel::default()));
  }

  set_liquidity_factor{
    let info = AssetInfo::minimal(
      ChainAsset::Eth([1u8; 20]),
      FromStr::from_str("USDC/6").unwrap(),
    );
    assert_ok!(Cash::<T>::support_asset(RawOrigin::Root.into(), info));
  }: {
    assert_ok!(Cash::<T>::set_liquidity_factor(RawOrigin::Root.into(), ChainAsset::Eth([1u8; 20]), Factor(1u128)));
  }

  set_supply_cap{
    let info = AssetInfo::minimal(
      ChainAsset::Eth([1u8; 20]),
      FromStr::from_str("USDC/6").unwrap(),
    );
    assert_ok!(Cash::<T>::support_asset(RawOrigin::Root.into(), info));
  }: {
    assert_ok!(Cash::<T>::set_supply_cap(RawOrigin::Root.into(), ChainAsset::Eth([1u8; 20]), 1u128));
  }

  allow_next_code_with_hash {
    let new_code = vec![3u8; 100_000];
    let hash = <Ethereum as Chain>::hash_bytes(&new_code);
  }: {
    assert_eq!(Cash::<T>::allow_next_code_with_hash(RawOrigin::Root.into(), hash), Ok(()));
  }

  set_next_code_via_hash {
    let z in 10_000 .. 100_000;
    let new_code = vec![3u8; z.try_into().unwrap()];
    let hash = <Ethereum as Chain>::hash_bytes(&new_code);
    AllowedNextCodeHash::put(hash);
  }: {
    assert_eq!(Cash::<T>::set_next_code_via_hash(RawOrigin::None.into(), new_code), Ok(()));
  }

  // todo: parameterize over # vals?
  change_validators {
    let substrate_id: SubstrateId = [2; 32].into();
    let eth_address = [1; 20];
    let val_keys = vec![ValidatorKeys {
        substrate_id: substrate_id.clone(),
        eth_address: eth_address.clone(),
    }];
    // XXX really set it up using a transfer to validator first
    assert!(<T as Config>::AccountStore::insert(&substrate_id, ()).is_ok());
    assert_eq!(
      pallet_session::Module::<T>::set_keys(
        T::Origin::signed(substrate_id.into()),
        <T>::Keys::default(),
        vec![]
      ),
      Ok(())
    );
  }: {
    assert_eq!(Cash::<T>::change_validators(RawOrigin::Root.into(), val_keys), Ok(()));
  }

  exec_trx_request_extract {
    let signer_vec = <Ethereum as Chain>::signer_address().unwrap();
    let nonce: Nonce = 0u32.into();

    let extract_amt: i128 = MIN_TX_VALUE.try_into().unwrap();
    endow_tkn::<T>(signer_vec, extract_amt * 5, TKN_ADDR_BYTES);

    // amount, asset, account
    let raw_req: String = format!("(Extract {} Eth:{} Eth:{})", extract_amt, TKN_ADDR, ALICE_ADDRESS);
    let request_vec: Vec<u8> = raw_req.as_bytes().into();
    let prepended_request = format!("{}:{}", nonce, raw_req);

    let full_request: Vec<u8> =  format!("\x19Ethereum Signed Message:\n{}{}", prepended_request.len(), prepended_request).as_bytes().into();

    let eth_key_id = runtime_interfaces::validator_config_interface::get_eth_key_id().unwrap();
    let signature_raw = runtime_interfaces::keyring_interface::sign_one(full_request, eth_key_id).unwrap();
    let signature = ChainAccountSignature::Eth(signer_vec, signature_raw);

  }: {
    assert_eq!(Cash::<T>::exec_trx_request(RawOrigin::None.into(), request_vec, signature, nonce), Ok(()));
  }

  exec_trx_request_transfer {
    let signer_vec = <Ethereum as Chain>::signer_address().unwrap();
    let nonce: Nonce = 0u32.into();

    let transfer_amt: i128 = MIN_TX_VALUE.try_into().unwrap();
    endow_tkn::<T>(signer_vec, transfer_amt * 5, TKN_ADDR_BYTES);

    // max, asset, dest_acct
    let raw_req: String = format!("(Transfer {} Eth:{} Eth:{})", transfer_amt, TKN_ADDR, ALICE_ADDRESS);
    let request_vec: Vec<u8> = raw_req.as_bytes().into();
    let prepended_request = format!("{}:{}", nonce, raw_req);

    let full_request: Vec<u8> =  format!("\x19Ethereum Signed Message:\n{}{}", prepended_request.len(), prepended_request).as_bytes().into();

    let eth_key_id = runtime_interfaces::validator_config_interface::get_eth_key_id().unwrap();
    let signature_raw = runtime_interfaces::keyring_interface::sign_one(full_request, eth_key_id).unwrap();
    let signature = ChainAccountSignature::Eth(signer_vec, signature_raw);

  }: {
    assert_eq!(Cash::<T>::exec_trx_request(RawOrigin::None.into(), request_vec, signature, nonce), Ok(()));
  }

  exec_trx_request_liquidate {
    let signer_vec = <Ethereum as Chain>::signer_address().unwrap();
    let holder = ChainAccount::Eth(signer_vec);
    let nonce: Nonce = 0u32.into();
    let transfer_amt: i128 = MIN_TX_VALUE.try_into().unwrap();

    // bob supply tkn, transfer eth
    endow_tkn::<T>(BOB_ADDRESS_BYTES, transfer_amt * 5, TKN_ADDR_BYTES);
    endow_tkn::<T>(BOB_ADDRESS_BYTES, -transfer_amt * 5, ETH_BYTES);

    // alice supply some collateral, liquidate
    endow_tkn::<T>(signer_vec, transfer_amt * 5, [2; 20]);
    let raw_req: String = format!("(Liquidate {} Eth:{} Eth:{} Eth:{})", MIN_TX_VALUE, ETH_ADDR, TKN_ADDR, BOB_ADDRESS);
    let request_vec: Vec<u8> = raw_req.as_bytes().into();
    let prepended_request = format!("{}:{}", nonce, raw_req);
    let full_request: Vec<u8> = format!("\x19Ethereum Signed Message:\n{}{}", prepended_request.len(), prepended_request).as_bytes().into();
    let eth_key_id = runtime_interfaces::validator_config_interface::get_eth_key_id().unwrap();
    let signature_raw = runtime_interfaces::keyring_interface::sign_one(full_request, eth_key_id).unwrap();
    let signature = ChainAccountSignature::Eth(signer_vec, signature_raw);
  }: {
    assert_eq!(Cash::<T>::exec_trx_request(RawOrigin::None.into(), request_vec, signature, nonce), Ok(()));
  }
}

impl_benchmark_test_suite!(
	Cash,
	crate::tests::new_test_ext(),
	crate::tests::Test,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        initialize_storage,
        mock::{new_test_ext, Test},
    };

    #[test]
    fn test_benchmarks() {
        new_test_ext().execute_with(|| {
            initialize_storage();
            assert_ok!(test_benchmark_on_initialize::<Test>());
            // assert_ok!(test_benchmark_receive_chain_blocks::<Test>());
            assert_ok!(test_benchmark_publish_signature::<Test>());
            assert_ok!(test_benchmark_set_yield_next::<Test>());
            assert_ok!(test_benchmark_support_asset::<Test>());
            assert_ok!(test_benchmark_set_rate_model::<Test>());
            assert_ok!(test_benchmark_set_liquidity_factor::<Test>());
            assert_ok!(test_benchmark_set_supply_cap::<Test>());
            assert_ok!(test_benchmark_allow_next_code_with_hash::<Test>());
            assert_ok!(test_benchmark_set_next_code_via_hash::<Test>());
            assert_ok!(test_benchmark_change_validators::<Test>());
            assert_ok!(test_benchmark_exec_trx_request_extract::<Test>());
            assert_ok!(test_benchmark_exec_trx_request_transfer::<Test>());
            assert_ok!(test_benchmark_exec_trx_request_liquidate::<Test>());
        });
    }
}
