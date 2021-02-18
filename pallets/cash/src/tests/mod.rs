#![allow(non_upper_case_globals)]

use crate::{chains::*, core::*, factor::*, mock::*, rates::*, reason::*, symbol::*, types::*, *};
use codec::{Decode, Encode};
use hex_literal::hex;
use our_std::str::FromStr;
use sp_core::crypto::AccountId32;
use sp_core::offchain::testing;

pub use frame_support::{assert_err, assert_ok, dispatch::DispatchError};

pub mod protocol;

pub const ETH: Units = Units::from_ticker_str("ETH", 18);
pub const Eth: ChainAsset = ChainAsset::Eth(hex!("EeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE"));
pub const _eth: AssetInfo = AssetInfo {
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
    runtime_interfaces::set_validator_config_dev_defaults();
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
    CashModule::initialize_reporters(
        vec![
            "0x85615b076615317c80f14cbad6501eec031cd51c",
            "0xfCEAdAFab14d46e20144F48824d0C09B1a03F2BC",
        ]
        .try_into()
        .unwrap(),
    );
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
}

#[test]
fn it_fails_exec_trx_request_signed() {
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
fn process_eth_event_happy_path() {
    new_test_ext().execute_with(|| {
        initialize_storage();
        // Dispatch a signed extrinsic.
        let event_id = ChainLogId::Eth(3858223, 0);
        let event = ChainLogEvent::Eth(ethereum_client::EthereumLogEvent {
            block_hash: [3; 32],
            block_number: 3858223,
            transaction_index: 0,
            log_index: 0,
            event: ethereum_client::EthereumEvent::Lock {
                asset: [1; 20],
                holder: [2; 20],
                amount: 10,
            },
        });
        let payload = event.encode();
        let signature = <Ethereum as Chain>::sign_message(&payload).unwrap(); // Sign with our "shared" private key for now XXX

        assert_ok!(CashModule::receive_event(
            Origin::none(),
            event_id,
            event,
            signature
        ));

        match CashModule::event_state(event_id) {
            EventState::Pending { signers } => {
                assert_eq!(signers.len(), 1);
                assert_eq!(
                    hex::encode(signers[0]),
                    "6a72a2f14577d9cd0167801efdd54a07b40d2b61"
                );
            }
            _ => {
                assert!(false);
            }
        }
    });
}

#[test]
fn process_eth_event_fails_for_bad_signature() {
    new_test_ext().execute_with(|| {
        let event_id = ChainLogId::Eth(3858223, 0);
        let event = ChainLogEvent::Eth(ethereum_client::EthereumLogEvent {
            block_hash: [3; 32],
            block_number: 3858223,
            transaction_index: 0,
            log_index: 0,
            event: ethereum_client::EthereumEvent::Lock {
                asset: [1; 20],
                holder: [2; 20],
                amount: 10,
            },
        });

        // Dispatch a signed extrinsic.
        assert_err!(
            CashModule::receive_event(Origin::signed(Default::default()), event_id, event, [0; 65]),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn process_eth_event_fails_if_not_validator() {
    new_test_ext().execute_with(|| {
        let event_id = ChainLogId::Eth(3858223, 0);
        let event = ChainLogEvent::Eth(ethereum_client::EthereumLogEvent {
            block_hash: [3; 32],
            block_number: 3858223,
            transaction_index: 0,
            log_index: 0,
            event: ethereum_client::EthereumEvent::Lock {
                asset: [1; 20],
                holder: [2; 20],
                amount: 10,
            },
        });

        initialize_storage();
        let sig = [
            238, 16, 209, 247, 127, 204, 225, 110, 235, 0, 62, 178, 92, 3, 21, 98, 228, 151, 49,
            101, 43, 60, 18, 190, 2, 53, 127, 122, 190, 161, 216, 207, 5, 8, 141, 244, 66, 182,
            118, 138, 220, 196, 6, 153, 77, 35, 141, 6, 78, 46, 97, 167, 242, 188, 141, 102, 167,
            209, 126, 30, 123, 73, 238, 34, 28,
        ];
        assert_err!(
            CashModule::receive_event(Origin::none(), event_id, event, sig),
            Reason::UnknownValidator
        );
    });
}

const TEST_OPF_URL: &str = "http://localhost/";

#[test]
fn test_process_prices_happy_path_makes_required_http_call() {
    std::env::set_var("OPF_URL", TEST_OPF_URL);
    let calls: Vec<testing::PendingRequest> = vec![testing::PendingRequest {
        method: "GET".into(),
        uri: TEST_OPF_URL.into(),
        body: vec![],
        response: Some(
            internal::oracle::tests::API_RESPONSE_TEST_DATA
                .to_owned()
                .into_bytes(),
        ),
        headers: vec![],
        sent: true,
        ..Default::default()
    }];

    let (mut t, _pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);
    t.execute_with(|| {
        initialize_storage();
        assert_ok!(internal::oracle::process_prices::<Test>(1u64));
        // sadly, it seems we can not check storage here, but we should at least be able to check that
        // the OCW attempted to call the post_price extrinsic.. that is a todo XXX
    });
}

#[test]
fn test_post_price_happy_path() {
    // an eth price message
    let test_payload = hex::decode("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000002baa48a00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000").unwrap();
    let test_signature = hex::decode("41a3f89a526dee766049f3699e9e975bfbabda4db677c9f5c41fbcc0730fccb84d08b2208c4ffae0b87bb162e2791cc305ee4e9a1d936f9e6154356154e9a8e9000000000000000000000000000000000000000000000000000000000000001c").unwrap();
    new_test_ext().execute_with(|| {
        initialize_storage(); // sets up ETH
        CashModule::post_price(Origin::none(), test_payload, test_signature).unwrap();
        let eth_price = CashModule::price(ETH.ticker);
        let eth_price_time = CashModule::price_time(ETH.ticker);
        assert_eq!(eth_price, Some(732580000));
        assert_eq!(eth_price_time, Some(1609340760000));
    });
}

#[test]
fn test_post_price_invalid_signature() {
    // an eth price message
    let test_payload = hex::decode("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000002baa48a00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000").unwrap();
    let test_signature = hex::decode("41a3f89a526dee766049f3699e9e975bfbabda4db677c9f5c41fbcc0730fccb84d08b2208c4ffae0b87bb162e2791cc305ee4e9a1d936f9e6154356154e9a8e90000000000000000000000000000000000000000000000000000000000000003").unwrap();
    new_test_ext().execute_with(|| {
        initialize_storage(); // sets up ETH
        let result = CashModule::post_price(Origin::none(), test_payload, test_signature);
        assert_err!(
            result,
            Reason::CryptoError(compound_crypto::CryptoError::RecoverError)
        );
    });
}

#[test]
fn test_post_price_invalid_reporter() {
    // an eth price message
    let test_payload = hex::decode("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000002baa48a00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000").unwrap();
    let test_signature = hex::decode("51a3f89a526dee766049f3699e9e975bfbabda4db677c9f5c41fbcc0730fccb84d08b2208c4ffae0b87bb162e2791cc305ee4e9a1d936f9e6154356154e9a8e9000000000000000000000000000000000000000000000000000000000000001c").unwrap();
    new_test_ext().execute_with(|| {
        initialize_storage(); // sets up ETH
        let result = CashModule::post_price(Origin::none(), test_payload, test_signature);
        assert_err!(
            result,
            Reason::CryptoError(compound_crypto::CryptoError::RecoverError)
        );
        // XXX is this testing the right thing?
        //  should it be:
        // assert_err!(result, Reason::OracleError(OracleError::NotAReporter)); ??
    });
}

#[test]
fn test_post_price_stale_price() {
    // an eth price message
    let test_payload = hex::decode("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000002baa48a00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000").unwrap();
    let test_signature = hex::decode("41a3f89a526dee766049f3699e9e975bfbabda4db677c9f5c41fbcc0730fccb84d08b2208c4ffae0b87bb162e2791cc305ee4e9a1d936f9e6154356154e9a8e9000000000000000000000000000000000000000000000000000000000000001c").unwrap();
    new_test_ext().execute_with(|| {
        initialize_storage(); // sets up ETH
                              // post once
        CashModule::post_price(Origin::none(), test_payload.clone(), test_signature.clone())
            .unwrap();
        let eth_price = CashModule::price(ETH.ticker);
        let eth_price_time = CashModule::price_time(ETH.ticker);
        assert_eq!(eth_price, Some(732580000));
        assert_eq!(eth_price_time, Some(1609340760000));
        // try to post the same thing again
        let result = CashModule::post_price(Origin::none(), test_payload, test_signature);
        assert_err!(result, Reason::OracleError(OracleError::StalePrice));
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

#[test]
fn offchain_worker_test() {
    use frame_support::traits::OffchainWorker;
    std::env::set_var("OPF_URL", TEST_OPF_URL);
    let mut calls: Vec<testing::PendingRequest> =
        events::tests::get_mockup_http_calls(testdata::json_responses::EVENTS_RESPONSE.to_vec());
    let price_call = testing::PendingRequest {
        method: "GET".into(),
        uri: TEST_OPF_URL.into(),
        body: vec![],
        response: Some(
            internal::oracle::tests::API_RESPONSE_TEST_DATA
                .to_owned()
                .into_bytes(),
        ),
        headers: vec![],
        sent: true,
        ..Default::default()
    };

    calls.push(price_call);

    let (mut t, pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);

    t.execute_with(|| {
        initialize_storage();

        // Set block number
        let block = 1;
        System::set_block_number(block);

        // Execute offchain worker with no cached block number
        CashModule::offchain_worker(block);

        // Check that length equals 3 + 8 (3 Lock events and 8 oracle prices)
        assert_eq!(pool_state.read().transactions.len(), 11);

        // Check `post_price` transactions
        let messages = [
            "0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000688e4cda00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000002baa48a00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000f51180000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034441490000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000057e400000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000035a52580000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000321900000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034241540000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000c63e00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034b4e430000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000ad33d80000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000044c494e4b00000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000009206d00000000000000000000000000000000000000000000000000000000000000000670726963657300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004434f4d5000000000000000000000000000000000000000000000000000000000"
        ];

        let signatures = [
            "69538bfa1a2097ea206780654d7baac3a17ee57547ee3eeb5d8bcb58a2fcdf401ff8834f4a003193f24224437881276fe76c8e1c0a361081de854457d41d0690000000000000000000000000000000000000000000000000000000000000001c",
            "41a3f89a526dee766049f3699e9e975bfbabda4db677c9f5c41fbcc0730fccb84d08b2208c4ffae0b87bb162e2791cc305ee4e9a1d936f9e6154356154e9a8e9000000000000000000000000000000000000000000000000000000000000001c",
            "15a9e7019f2b45c5e64646df571ea944b544dbf9093fbe19e41afea68fa58d721e53449245eebea3f351dbdff4dff09cf303a335cb4455f0d3219f308d448483000000000000000000000000000000000000000000000000000000000000001c",
            "25be45b4fa82f48160cb0218acafe26e6fea2be797710add737d09ad305ab54e5f75783b857b2c5c526acb3f9b34ffba64c1251843d320f04b5c0efbbe661d17000000000000000000000000000000000000000000000000000000000000001b",
            "19984214a69bccb410910de3b277d19fd86f2524510d83b4fc139f1469b11e375297ea89aeda2bceda4a4553e7815f93d3cff192ade88dccf43fb18ba73a97a7000000000000000000000000000000000000000000000000000000000000001b",
            "549e608b0e2acc98a36ac88fac610909d430b89c7501183d83c05189260baa6754b16ef74c804f7a7789e4e468878bfe153d76a7029c29f9acce86942a1ff492000000000000000000000000000000000000000000000000000000000000001c",
            "01612605d0de98506ced9ca0414a08b7c335cd1dfa0ea2b62d283a2e27d8d33c25eb0abd6cc2625d950f59baf3300a71e269c3f3eea81e5ed8876bb2f4e75cfd000000000000000000000000000000000000000000000000000000000000001b",
            "883317a2aa03f1523e95bedb961d7aabfbfba73bb9f54685639d0bc1eb2fd16a7c5510e7f68e1e0824bd5a96093ef921aabb36f79e89defc4d216f6dc0d79fbb000000000000000000000000000000000000000000000000000000000000001b"
        ];
        for _ in 0..8 {
            let tx = pool_state.write().transactions.pop().unwrap();
            let ex: Extrinsic = Decode::decode(&mut &*tx).unwrap();
            if let mock::Call::Cash(crate::Call::post_price(msg, sig)) = ex.call {
                let msg_str: &str = &hex::encode(&msg)[..];
                let sig_str: &str = &hex::encode(&sig)[..];
                assert!(messages.contains(&msg_str));
                assert!(signatures.contains(&sig_str));
            } else {
                assert!(false);
            }
        }

        // Check `receive_event` transactions
        let tx1 = pool_state.write().transactions.pop().unwrap();
        let ex1: Extrinsic = Decode::decode(&mut &*tx1).unwrap();

        let tx2 = pool_state.write().transactions.pop().unwrap();
        let ex2: Extrinsic = Decode::decode(&mut &*tx2).unwrap();

        let tx3 = pool_state.write().transactions.pop().unwrap();
        let ex3: Extrinsic = Decode::decode(&mut &*tx3).unwrap();

        assert_eq!(ex1.signature, None);
        assert_eq!(ex2.signature, None);
        assert_eq!(ex3.signature, None);

        if let mock::Call::Cash(crate::Call::receive_event(event_id, event, _signature)) = ex1.call {
            assert_eq!(event_id, ChainLogId::Eth(3932939, 14)); // TODO: Should this be trx index or log_index?
            assert_eq!(event, ChainLogEvent::Eth(ethereum_client::EthereumLogEvent {
                block_hash: [164, 169, 110, 149, 119, 24, 227, 163, 11, 119, 166, 103, 249, 57, 120, 216, 244, 56, 189, 205, 86, 255, 3, 84, 95, 8, 200, 51, 217, 162, 102, 135],
                block_number: 3932939,
                transaction_index: 4,
                log_index: 14,
                event: ethereum_client::EthereumEvent::Lock {
                    asset: [228, 232, 31, 166, 177, 99, 39, 212, 183, 140, 254, 184, 58, 173, 224, 75, 167, 7, 81, 101],
                    holder: [254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56, 125, 95, 244, 112, 49],
                    amount: 100000000000000000000,
                },
            }));
        } else {
            assert!(false);
        }

        if let mock::Call::Cash(crate::Call::receive_event(event_id, event, _signature)) = ex2.call {
            assert_eq!(event_id, ChainLogId::Eth(3932897, 1));
            assert_eq!(event, ChainLogEvent::Eth(ethereum_client::EthereumLogEvent {
                block_hash: [165, 200, 2, 78, 105, 154, 92, 48, 235, 150, 94, 71, 181, 21, 124, 6, 199, 111, 59, 114, 107, 255, 55, 122, 10, 83, 51, 165, 97, 242, 86, 72],
                block_number: 3932897,
                transaction_index: 0,
                log_index: 1,
                event: ethereum_client::EthereumEvent::Lock {
                    asset: [216, 123, 167, 165, 11, 46, 126, 102, 15, 103, 138, 137, 94, 75, 114, 231, 203, 76, 205, 156],
                    holder: [254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56, 125, 95, 244, 112, 49],
                    amount: 100000000,
                },
            }));
        } else {
            assert!(false);
        }

        if let mock::Call::Cash(crate::Call::receive_event(event_id, event, _signature)) = ex3.call {
            assert_eq!(event_id, ChainLogId::Eth(3858223, 0));
            assert_eq!(event, ChainLogEvent::Eth(ethereum_client::EthereumLogEvent {
                block_hash: [193, 192, 235, 55, 181, 105, 35, 173, 158, 32, 253, 179, 28, 168, 130, 152, 141, 82, 23, 247, 202, 36, 182, 41, 124, 166, 237, 112, 8, 17, 207, 35],
                block_number: 3858223,
                transaction_index: 0,
                log_index: 0,
                event: ethereum_client::EthereumEvent::Lock {
                    asset: [238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238],
                    holder: [81, 60, 31, 244, 53, 236, 206, 221, 15, 218, 94, 221, 42, 213, 229, 70, 31, 14, 135, 38],
                    amount: 5000000000000000,
                },
            }));
        } else {
            assert!(false);
        }
    });
}
