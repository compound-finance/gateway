use crate::types::SignersSet;
use crate::{chains::*, core::*, internal, mock::*, oracle::*, rates::*, reason::*, symbol::*, *};
use codec::{Decode, Encode};
use frame_support::{assert_err, assert_ok, dispatch::DispatchError};
use our_std::collections::btree_set::BTreeSet;
use our_std::iter::FromIterator;
use our_std::str::FromStr;
use sp_core::crypto::AccountId32;
use sp_core::offchain::testing;

#[cfg(test)]
mod ocw;

pub const ETH: Units = Units::from_ticker_str("ETH", 18);

pub fn eth_asset() -> ChainAsset {
    ChainAsset::Eth(
        <Ethereum as Chain>::str_to_address("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE").unwrap(),
    )
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

pub fn initialize_storage() {
    runtime_interfaces::set_validator_config_dev_defaults();
    CashModule::initialize_assets(vec![
        AssetInfo {
            liquidity_factor: FromStr::from_str("7890").unwrap(),
            ..AssetInfo::minimal(
                FromStr::from_str("eth:0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE").unwrap(),
                FromStr::from_str("ETH/18").unwrap(),
            )
            .unwrap()
        },
        AssetInfo {
            ticker: FromStr::from_str("USD").unwrap(),
            liquidity_factor: FromStr::from_str("7890").unwrap(),
            ..AssetInfo::minimal(
                FromStr::from_str("eth:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap(),
                FromStr::from_str("USDC/6").unwrap(),
            )
            .unwrap()
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
fn process_eth_event_happy_path() {
    new_test_ext().execute_with(|| {
        initialize_storage();
        // Dispatch a signed extrinsic.
        let chain_id = ChainId::Eth;
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
            chain_id,
            event_id,
            event,
            signature
        ));

        assert_eq!(
            CashModule::pending_events(chain_id, event_id),
            BTreeSet::from_iter(vec![hex::decode(
                "6a72a2f14577d9cd0167801efdd54a07b40d2b61"
            )
            .unwrap()
            .try_into()
            .unwrap()])
        );

        assert_eq!(
            CashModule::done_events(chain_id, event_id),
            BTreeSet::from_iter(vec![])
        );

        assert_eq!(CashModule::failed_events(chain_id, event_id), Reason::None);
    });
}

#[test]
fn process_eth_event_fails_for_bad_signature() {
    new_test_ext().execute_with(|| {
        let event_id = ChainLogId::Eth(3858223, 0);
        let chain_id = ChainId::Eth;
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
            CashModule::receive_event(
                Origin::signed(Default::default()),
                chain_id,
                event_id,
                event,
                [0; 65]
            ),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn process_eth_event_fails_if_not_validator() {
    new_test_ext().execute_with(|| {
        let event_id = ChainLogId::Eth(3858223, 0);
        let chain_id = ChainId::Eth;
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
            CashModule::receive_event(Origin::none(), chain_id, event_id, event, sig),
            Reason::UnknownValidator
        );
    });
}

#[test]
fn correct_error_for_none_value() {
    // XXX keep as example for now
    // new_test_ext().execute_with(|| {
    //     // Ensure the expected error is thrown when no value is present.
    //     assert_noop!(
    //         CashModule::cause_error(Origin::signed(Default::default())),
    //         Error::<Test>::NoneValue
    //     );
    // });
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
            crate::oracle::tests::API_RESPONSE_TEST_DATA
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
        internal::oracle::process_prices::<Test>(1u64).unwrap();
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
        assert_eq!(eth_price_time, Some(1609340760));
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
        assert_eq!(eth_price_time, Some(1609340760));
        // try to post the same thing again
        let result = CashModule::post_price(Origin::none(), test_payload, test_signature);
        assert_err!(result, Reason::OracleError(OracleError::StalePrice));
    });
}

#[test]
fn test_set_interest_rate_model() {
    new_test_ext().execute_with(|| {
        initialize_storage();
        let asset = eth_asset();
        let expected_model = InterestRateModel::new_kink(100, 101, 5000, 202);
        CashModule::set_rate_model(Origin::root(), asset.clone(), expected_model).unwrap();
        let asset_info = CashModule::asset(asset).expect("no asset");
        assert_eq!(asset_info.rate_model, expected_model);
    });
}
