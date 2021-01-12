use crate::{chains::*, core::*, mock::*, *};
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchError};
use sp_core::offchain::testing;

const ETH: Symbol = Symbol(
    ['E', 'T', 'H', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
    18,
); // XXX macro?

fn andrew() -> GenericAccount {
    (ChainId::Eth, [123; 20].to_vec())
}

#[test]
fn it_fails_magic_extract_signed() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        assert_err!(
            CashModule::magic_extract(Origin::signed(Default::default()), andrew(), 42u32.into()),
            DispatchError::BadOrigin
        );
        // Read pallet storage and assert an expected result.
        assert_eq!(CashModule::cash_balance(andrew()), None);
    });
}

#[test]
fn it_magically_extracts() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        assert_ok!(CashModule::magic_extract(
            Origin::none(),
            andrew(),
            42u32.into()
        ));
        // Read pallet storage and assert an expected result.
        assert_eq!(CashModule::cash_balance(andrew()), Some(42u32.into()));

        // Dispatch a second extrinsic.
        assert_ok!(CashModule::magic_extract(
            Origin::none(),
            andrew(),
            42u32.into()
        ));
        // Read pallet storage and assert an expected result.
        assert_eq!(CashModule::cash_balance(andrew()), Some(84u32.into()));
    });
}

fn initialize_storage() {
    CashModule::initialize_validators(vec![
        "0458bfa2eec1cd8f451b41a1ad1034614986a6e65eabe24b5a7888d3f7422d6130e35d36561b207b1f9462bd8a982bd5b5204a2f8827b38469841ef537554ff1ba".into(),
        "04c3e5ff2cb194d58e6a51ffe2df490c70d899fee4cdfff0a834fcdfd327a1d1bdaae3f1719d7fd9a9ee4472aa5b14e861adef01d9abd44ce82a85e19d6e21d3a4".into()
    ]);
    CashModule::initialize_reporters(vec![
        "85615b076615317c80f14cbad6501eec031cd51c".into(),
        "fCEAdAFab14d46e20144F48824d0C09B1a03F2BC".into(),
    ]);
    CashModule::initialize_symbols(vec!["ETH".into()]); // XXX fixme + needs decimals
    CashModule::initialize_price_key_mapping(vec!["USDC:ETH:deadbeef".into()]);
}

#[test]
fn process_eth_event_happy_path() {
    new_test_ext().execute_with(|| {
        initialize_storage();
        // Dispatch a signed extrinsic.
        // XXX
        let payload = vec![
            47u8, 223, 58, 0, 0, 0, 0, 0, 0, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238,
            238, 238, 238, 238, 238, 238, 238, 238, 238, 81, 60, 31, 244, 53, 236, 206, 221, 15,
            218, 94, 221, 42, 213, 229, 70, 31, 14, 135, 38, 0, 128, 224, 55, 121, 195, 17, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
        ];

        let sig = [
            228, 180, 56, 220, 198, 16, 107, 231, 10, 157, 165, 109, 245, 75, 46, 66, 164, 47, 161,
            71, 119, 142, 174, 183, 246, 102, 9, 121, 89, 21, 104, 174, 21, 202, 66, 26, 78, 204,
            163, 35, 125, 113, 170, 242, 7, 213, 238, 201, 16, 22, 61, 174, 1, 22, 128, 224, 221,
            97, 133, 205, 126, 99, 4, 105, 1,
        ];

        assert_ok!(CashModule::process_eth_event(
            Origin::signed(Default::default()),
            payload,
            sig
        ));
        // Read pallet storage and assert an expected result.
        // XXX assert_eq!(CashModule::something(), Some(42));
    });
}

#[test]
fn it_fails_for_bad_signature() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        assert_err!(
            CashModule::process_eth_event(Origin::signed(Default::default()), vec![], [0; 65]),
            Error::<Test>::SignedPayloadError
        );
        // Read pallet storage and assert an expected result.
        // XXX assert_eq!(CashModule::something(), Some(42));
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

#[test]
fn test_process_open_price_feed_happy_path_makes_required_http_call() {
    let calls: Vec<testing::PendingRequest> = vec![testing::PendingRequest {
        method: "GET".into(),
        uri: crate::oracle::OKEX_OPEN_PRICE_FEED_URL.into(),
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

    new_test_ext_with_http_calls(calls).execute_with(|| {
        initialize_storage();
        CashModule::process_open_price_feed(1u64).unwrap();
        // sadly, it seems we can not check storage here, but we should at least be able to check that
        // the OCW attempted to call the post_price extrinsic.. that is a todo
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
        let eth_price = CashModule::prices(ETH);
        let eth_price_time = CashModule::price_times(ETH);
        assert_eq!(eth_price, 732580000);
        assert_eq!(eth_price_time, 1609340760);
    });
}

#[test]
fn test_post_price_invalid_signature() {
    // an eth price message
    let test_payload = hex::decode("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000002baa48a00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000").unwrap();
    let test_signature = hex::decode("41a3f89a526dee766049f3699e9e975bfbabda4db677c9f5c41fbcc0730fccb84d08b2208c4ffae0b87bb162e2791cc305ee4e9a1d936f9e6154356154e9a8e900000000000000000000000000000000000000000000000000000000000000ff").unwrap();
    new_test_ext().execute_with(|| {
        initialize_storage(); // sets up ETH
        let result = CashModule::post_price(Origin::none(), test_payload, test_signature);
        assert_err!(result, Error::<Test>::OpenOracleErrorInvalidSignature);
    });
}

fn get_test_keyring() {}

#[test]
fn test_post_price_invalid_reporter() {
    // an eth price message
    let test_payload = hex::decode("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000002baa48a00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000").unwrap();
    let test_signature = hex::decode("51a3f89a526dee766049f3699e9e975bfbabda4db677c9f5c41fbcc0730fccb84d08b2208c4ffae0b87bb162e2791cc305ee4e9a1d936f9e6154356154e9a8e9000000000000000000000000000000000000000000000000000000000000001c").unwrap();
    new_test_ext().execute_with(|| {
        initialize_storage(); // sets up ETH
        let result = CashModule::post_price(Origin::none(), test_payload, test_signature);
        assert_err!(result, Error::<Test>::OpenOracleErrorInvalidSignature);
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
        let eth_price = CashModule::prices(ETH);
        let eth_price_time = CashModule::price_times(ETH);
        assert_eq!(eth_price, 732580000);
        assert_eq!(eth_price_time, 1609340760);
        // try to post the same thing again
        let result = CashModule::post_price(Origin::none(), test_payload, test_signature);
        assert_err!(result, Error::<Test>::OpenOracleErrorStalePrice);
    });
}
