use crate::{chains::*, core::*, mock::*, symbol::*, *};
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchError};
use sp_core::offchain::testing;

const ETH: Symbol = Symbol(
    ['E', 'T', 'H', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
    18,
); // XXX macro?

fn andrew() -> ChainAccount {
    ChainAccount::Eth([123; 20])
}

#[test]
fn it_fails_exec_trx_request_signed() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        assert_err!(
            CashModule::exec_trx_request(
                Origin::signed(Default::default()),
                vec![],
                ChainSignature::Eth([0; 65])
            ),
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
        assert_ok!(magic_extract_internal::<Test>(
            andrew(),
            andrew(),
            42u128.into()
        ));
        // Read pallet storage and assert an expected result.
        assert_eq!(CashModule::cash_balance(andrew()), Some(42u128.into()));

        // Dispatch a second extrinsic.
        assert_ok!(magic_extract_internal::<Test>(
            andrew(),
            andrew(),
            42u128.into()
        ));
        // Read pallet storage and assert an expected result.
        assert_eq!(CashModule::cash_balance(andrew()), Some(84u128.into()));
    });
}

fn initialize_storage() {
    CashModule::initialize_validators(vec![
        "6a72a2f14577D9Cd0167801EFDd54a07B40d2b61".into(), // pk: 50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374
        "58547bfa800b08a61b4adacbb78664bba2cb9301".into(),
    ]);
    CashModule::initialize_reporters(vec![
        "85615b076615317c80f14cbad6501eec031cd51c".into(),
        "fCEAdAFab14d46e20144F48824d0C09B1a03F2BC".into(),
    ]);
    CashModule::initialize_symbols(vec!["ETH".into()]); // XXX fixme + needs decimals
    CashModule::initialize_price_key_mapping(vec![
        "USDC:ETH:EeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE".into(),
    ]);
}

#[test]
fn process_eth_event_happy_path() {
    new_test_ext().execute_with(|| {
        initialize_storage();
        // Dispatch a signed extrinsic.
        // XXX
        let payload = hex::decode("2fdf3a000000000000eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee513c1ff435eccedd0fda5edd2ad5e5461f0e87260080e03779c311000000000000000000").unwrap();
        let sig = chains::eth::sign(&payload); // Sign with our "shared" private key for now

        assert_ok!(CashModule::process_eth_event(Origin::none(), payload, sig));
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
            DispatchError::BadOrigin
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
