#![allow(non_upper_case_globals)]

use crate::{
    chains::*, core::*, factor::*, notices::*, rates::*, reason::*, symbol::*, types::*, *,
};

use our_std::collections::btree_set::BTreeSet;

use codec::{Decode, Encode};
use hex_literal::hex;
use sp_core::crypto::AccountId32;
use sp_core::offchain::testing;
use test_env_log::test;

pub use frame_support::{assert_err, assert_ok, dispatch::DispatchError};
pub use our_std::{iter::FromIterator, str::FromStr};

pub use mock::*;
pub mod mock;

pub mod extract;
pub mod protocol;
pub mod testdata;
pub mod worker;

pub const ETH: Units = Units::from_ticker_str("ETH", 18);
pub const Eth: ChainAsset = ChainAsset::Eth(hex!("EeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE"));
pub const eth: AssetInfo = AssetInfo {
    asset: Eth,
    decimals: ETH.decimals,
    liquidity_factor: LiquidityFactor::from_nominal("0.8"),
    rate_model: InterestRateModel::Kink {
        zero_rate: APR(0),
        kink_rate: APR(200),
        kink_utilization: Factor::from_nominal("0.9"),
        full_rate: APR(5000),
    },
    miner_shares: Factor::from_nominal("0.05"),
    supply_cap: Quantity::from_nominal("1000", ETH).value,
    symbol: Symbol(ETH.ticker.0),
    ticker: Ticker(ETH.ticker.0),
};

pub const UNI: Units = Units::from_ticker_str("UNI", 18);
pub const Uni: ChainAsset = ChainAsset::Eth(hex!("1f9840a85d5af5bf1d1762f925bdaddc4201f984"));
pub const uni: AssetInfo = AssetInfo {
    asset: Uni,
    decimals: UNI.decimals,
    liquidity_factor: LiquidityFactor::from_nominal("0.7"),
    rate_model: InterestRateModel::Kink {
        zero_rate: APR(0),
        kink_rate: APR(500),
        kink_utilization: Factor::from_nominal("0.8"),
        full_rate: APR(2000),
    },
    miner_shares: Factor::from_nominal("0.05"),
    supply_cap: Quantity::from_nominal("1000", UNI).value,
    symbol: Symbol(UNI.ticker.0),
    ticker: Ticker(UNI.ticker.0),
};

#[macro_export]
macro_rules! bal {
    ($string:expr, $units:expr) => {
        Balance::from_nominal($string, $units);
    };
}

#[macro_export]
macro_rules! qty {
    ($string:expr, $units:expr) => {
        Quantity::from_nominal($string, $units);
    };
}

pub fn initialize_storage() {
    pallet_oracle::Module::<Test>::initialize_reporters(
        vec![
            "0x85615b076615317c80f14cbad6501eec031cd51c",
            "0xfCEAdAFab14d46e20144F48824d0C09B1a03F2BC",
        ]
        .try_into()
        .unwrap(),
    );

    CashModule::initialize_assets(vec![
        AssetInfo {
            liquidity_factor: FromStr::from_str("7890").unwrap(),
            ..AssetInfo::minimal(
                FromStr::from_str("eth:0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE").unwrap(),
                FromStr::from_str("ETH/18").unwrap(),
            )
        },
        AssetInfo {
            ticker: FromStr::from_str("USD").unwrap(),
            liquidity_factor: FromStr::from_str("7890").unwrap(),
            ..AssetInfo::minimal(
                FromStr::from_str("eth:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap(),
                FromStr::from_str("USDC/6").unwrap(),
            )
        },
    ]);

    CashModule::initialize_validators(vec![
        ValidatorKeys {
            substrate_id: AccountId32::from_str("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY")
                .unwrap(),
            eth_address: <Ethereum as Chain>::str_to_address(
                "0x6a72a2f14577D9Cd0167801EFDd54a07B40d2b61",
            )
            .unwrap(), // pk: 50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374
        },
        ValidatorKeys {
            substrate_id: AccountId32::from_str("5FfBQ3kwXrbdyoqLPvcXRp7ikWydXawpNs2Ceu3WwFdhZ8W4")
                .unwrap(),
            eth_address: <Ethereum as Chain>::str_to_address(
                "0x8ad1b2918c34ee5d3e881a57c68574ea9dbecb81",
            )
            .unwrap(),
        },
    ]);

    LastProcessedBlock::insert(
        ChainId::Eth,
        ChainBlock::Eth(ethereum_client::EthereumBlock {
            hash: [
                97, 49, 76, 28, 104, 55, 225, 94, 96, 197, 182, 115, 47, 9, 33, 24, 221, 37, 227,
                236, 104, 31, 94, 8, 155, 58, 154, 210, 55, 78, 90, 138,
            ],
            parent_hash: [
                6, 46, 119, 220, 237, 67, 30, 182, 113, 165, 104, 57, 249, 109, 169, 18, 246, 141,
                132, 16, 36, 102, 87, 72, 211, 140, 211, 214, 121, 89, 97, 234,
            ],
            number: 1,
            events: vec![
                ethereum_client::EthereumEvent::Lock {
                    asset: [
                        238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238,
                        238, 238, 238, 238, 238,
                    ],
                    sender: [
                        254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56,
                        125, 95, 244, 112, 49,
                    ],
                    chain: String::from("ETH"),
                    recipient: [
                        81, 60, 31, 244, 53, 236, 206, 221, 15, 218, 94, 221, 42, 213, 229, 70, 31,
                        14, 135, 38, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    ],
                    amount: 1000000000000000000,
                },
                ethereum_client::EthereumEvent::Lock {
                    asset: [
                        216, 123, 167, 165, 11, 46, 126, 102, 15, 103, 138, 137, 94, 75, 114, 231,
                        203, 76, 205, 156,
                    ],
                    sender: [
                        254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56,
                        125, 95, 244, 112, 49,
                    ],
                    chain: String::from("ETH"),
                    recipient: [
                        254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56,
                        125, 95, 244, 112, 49, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    ],
                    amount: 1000000000000000000,
                },
                ethereum_client::EthereumEvent::Lock {
                    asset: [
                        228, 232, 31, 166, 177, 99, 39, 212, 183, 140, 254, 184, 58, 173, 224, 75,
                        167, 7, 81, 101,
                    ],
                    sender: [
                        254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56,
                        125, 95, 244, 112, 49,
                    ],
                    chain: String::from("ETH"),
                    recipient: [
                        254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56,
                        125, 95, 244, 112, 49, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    ],
                    amount: 1000000000000000000,
                },
            ],
        }),
    );
}

#[test]
fn test_it_fails_exec_trx_request_signed() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        assert_err!(
            CashModule::exec_trx_request(
                Origin::signed(Default::default()),
                vec![],
                ChainAccountSignature::Eth([0; 20], [0; 65]),
                0
            ),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn test_set_interest_rate_model() {
    new_test_ext().execute_with(|| {
        initialize_storage();
        let expected_model = InterestRateModel::new_kink(100, 101, 5000, 202);
        CashModule::set_rate_model(Origin::root(), Eth, expected_model).unwrap();
        let asset_info = CashModule::asset(Eth).expect("no asset");
        assert_eq!(asset_info.rate_model, expected_model);
    });
}
