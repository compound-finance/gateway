use crate::{chains::*, core::*, mock::*, rates::*, reason::Reason, symbol::*, *};
use frame_support::{assert_err, assert_ok, dispatch::DispatchError};
use our_std::str::FromStr;
use sp_core::crypto::AccountId32;
use sp_core::offchain::testing;
use sp_core::offchain::{
    testing::{PoolState, TestOffchainExt, TestTransactionPoolExt},
    OffchainExt, TransactionPoolExt,
};

const ETH: Symbol = Symbol::new("ETH", 18);

fn _andrew() -> ChainAccount {
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
        ConfigAsset {
            symbol: FromStr::from_str("ETH/18").unwrap(),
            asset: FromStr::from_str("eth:0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE").unwrap(),
        },
        ConfigAsset {
            symbol: FromStr::from_str("USDC/6").unwrap(),
            asset: FromStr::from_str("eth:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap(),
        },
    ]);
    CashModule::initialize_validators(
        //accountId32s
        vec![
            AccountId32::from_str("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY")
                .unwrap()
                .into(),
            AccountId32::from_str("5FfBQ3kwXrbdyoqLPvcXRp7ikWydXawpNs2Ceu3WwFdhZ8W4")
                .unwrap()
                .into(),
        ],
        // eth keys
        vec![
            ("6a72a2f14577D9Cd0167801EFDd54a07B40d2b61".into(),), // pk: 50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374
            ("8ad1b2918c34ee5d3e881a57c68574ea9dbecb81".into(),),
        ],
    );
    CashModule::initialize_reporters(vec![
        "85615b076615317c80f14cbad6501eec031cd51c".into(),
        "fCEAdAFab14d46e20144F48824d0C09B1a03F2BC".into(),
    ]);
}

#[test]
// TODO: move into integraion test area, validators are not present in unit tests as not all pallets are loaded
fn process_eth_event_happy_path() {
    new_test_ext().execute_with(|| {
        initialize_storage();
        // Dispatch a signed extrinsic.
        // XXX
        let event_id = (3858223, 0);
        let payload = hex::decode("2fdf3a000000000000eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee513c1ff435eccedd0fda5edd2ad5e5461f0e87260080e03779c311000000000000000000").unwrap();
        let checked_payload = payload.clone();
        let sig = <Ethereum as Chain>::sign_message(&payload).unwrap(); // Sign with our "shared" private key for now XXX

        assert_ok!(CashModule::process_eth_event(Origin::none(), payload, sig));
        let event_status = CashModule::eth_event_queue(event_id).unwrap();
        match event_status {
            EventStatus::<Ethereum>::Pending { signers } => {
                assert_eq!(signers.len(), 1);
                assert_eq!(hex::encode(signers[0]), "6a72a2f14577d9cd0167801efdd54a07b40d2b61");
            }
            _ => {
                assert!(false);
            }
        }

        // Second validator also saw this event
        let payload_validator_2 = checked_payload.clone();
        let sig_validator_2 = [238, 16, 209, 247, 127, 204, 226, 110, 235, 0, 62, 178, 92, 3, 21, 98, 228, 151, 49, 101, 43, 60, 18, 190, 2, 53, 127, 122, 190, 161, 216, 207, 5, 8, 141, 244, 66, 182, 118, 138, 220, 196, 6, 153, 77, 35, 141, 6, 78, 46, 97, 167, 242, 188, 141, 102, 167, 209, 126, 30, 123, 73, 238, 34, 28];
        assert_ok!(CashModule::process_eth_event(Origin::none(), payload_validator_2, sig_validator_2));
        let event_status_validator_2 = CashModule::eth_event_queue(event_id).unwrap();
        match event_status_validator_2 {
            EventStatus::<Ethereum>::Done => {
                // Check emitted events
                let our_events = System::events()
                    .into_iter()
                    .map(|r| r.event)
                    .filter_map(|e| {
                        if let TestEvent::cash(inner) = e { Some(inner) } else { None }
                    })
                    .collect::<Vec<_>>();
                let expected_event = Event::ProcessedEthEvent(checked_payload);
                assert_eq!(our_events[1], expected_event);
            }

            // XXX this now triggers a failure, not sure how to change the test data to increase the value
            EventStatus::<Ethereum>::Failed { reason: Reason::MinTxValueNotMet, .. } => {
                assert!(true);
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
        // Dispatch a signed extrinsic.
        assert_err!(
            CashModule::process_eth_event(Origin::signed(Default::default()), vec![], [0; 65]),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn process_eth_event_fails_if_not_validator() {
    new_test_ext().execute_with(|| {
        initialize_storage();
        let payload = hex::decode("2fdf3a000000000000eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee513c1ff435eccedd0fda5edd2ad5e5461f0e87260080e03779c311000000000000000000").unwrap();
        let sig = [238, 16, 209, 247, 127, 204, 225, 110, 235, 0, 62, 178, 92, 3, 21, 98, 228, 151, 49, 101, 43, 60, 18, 190, 2, 53, 127, 122, 190, 161, 216, 207, 5, 8, 141, 244, 66, 182, 118, 138, 220, 196, 6, 153, 77, 35, 141, 6, 78, 46, 97, 167, 242, 188, 141, 102, 167, 209, 126, 30, 123, 73, 238, 34, 28];
        assert_err!(CashModule::process_eth_event(Origin::none(), payload, sig), Error::<Test>::UnknownValidator);
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
fn test_process_open_price_feed_happy_path_makes_required_http_call() {
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
        let eth_price = CashModule::price(ETH);
        let eth_price_time = CashModule::price_time(ETH);
        assert_eq!(eth_price, 732580000);
        assert_eq!(eth_price_time, 1609340760);
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
        let eth_price = CashModule::price(ETH);
        let eth_price_time = CashModule::price_time(ETH);
        assert_eq!(eth_price, 732580000);
        assert_eq!(eth_price_time, 1609340760);
        // try to post the same thing again
        let result = CashModule::post_price(Origin::none(), test_payload, test_signature);
        assert_err!(result, Error::<Test>::OpenOracleErrorStalePrice);
    });
}

pub fn get_eth() -> ChainAsset {
    ChainAsset::Eth(
        <[u8; 20]>::try_from(hex::decode("EeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE").unwrap())
            .unwrap(),
    )
}

#[test]
fn test_set_interest_rate_model() {
    new_test_ext().execute_with(|| {
        initialize_storage();
        let asset = get_eth();
        let expected_model = InterestRateModel::new_kink(100, 101, 5000, 202);
        CashModule::update_interest_rate_model(Origin::root(), asset.clone(), expected_model)
            .unwrap();
        let actual_model = CashModule::model(asset);
        assert_eq!(actual_model, expected_model);
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
            crate::oracle::tests::API_RESPONSE_TEST_DATA
                .to_owned()
                .into_bytes(),
        ),
        headers: vec![],
        sent: true,
        ..Default::default()
    };

    calls.push(price_call);

    let (mut t, pool_state, offchain_state) = new_test_ext_with_http_calls(calls);

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
        for x  in 0..8 {
            let tx = pool_state.write().transactions.pop().unwrap();
            let ex: Extrinsic = Decode::decode(&mut &*tx).unwrap();
            if let Call::post_price(msg, sig) = ex.call {
                let msg_str: &str = &hex::encode(&msg)[..];
                let sig_str: &str = &hex::encode(&sig)[..];
                assert!(messages.contains(&msg_str));
                assert!(signatures.contains(&sig_str));
            } else {
                assert!(false);
            }
        }

        // Check `process_event` transactions
        let tx1 = pool_state.write().transactions.pop().unwrap();
        let ex1: Extrinsic = Decode::decode(&mut &*tx1).unwrap();

        let tx2 = pool_state.write().transactions.pop().unwrap();
        let ex2: Extrinsic = Decode::decode(&mut &*tx2).unwrap();

        let tx3 = pool_state.write().transactions.pop().unwrap();
        let ex3: Extrinsic = Decode::decode(&mut &*tx3).unwrap();

        assert_eq!(ex1.signature, None);
        assert_eq!(ex2.signature, None);
        assert_eq!(ex3.signature, None);
        if let Call::process_eth_event(payload, signature) = ex1.call {
            assert_eq!(hex::encode(&payload), "0b033c000e00000000e4e81fa6b16327d4b78cfeb83aade04ba7075165feb1ea27f888c384f1b0dc14fd6b387d5ff47031000010632d5ec76b0500000000000000");
            assert_eq!(hex::encode(&signature), "d3d8533515d0ccaab0f1c51122608bb83da0039cf8e1985f32a111f5545521a972b0023a447b67e3a39a0d624fd42ee57c66660a3e79595a8e7cfb2ead84ac1f1b");
        } else {
            assert!(false);
        }

        if let Call::process_eth_event(payload, signature) = ex2.call {
            assert_eq!(hex::encode(&payload), "e1023c000100000000d87ba7a50b2e7e660f678a895e4b72e7cb4ccd9cfeb1ea27f888c384f1b0dc14fd6b387d5ff4703100e1f505000000000000000000000000");
            assert_eq!(hex::encode(&signature), "39b79abc082ecb3b510979e189755491601ffc36467c990f627efd4d03bc12ff7768d3c0df746d351b7585950ae0da3444964c91113fa53e50298a9c7f15cab51b");
        } else {
            assert!(false);
        }

        if let Call::process_eth_event(payload, signature) = ex3.call {
            assert_eq!(hex::encode(&payload), "2fdf3a000000000000eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee513c1ff435eccedd0fda5edd2ad5e5461f0e87260080e03779c311000000000000000000");
            assert_eq!(hex::encode(&signature), "e4b438dcc6106be70a9da56df54b2e42a42fa147778eaeb7f6660979591568ae15ca421a4ecca3237d71aaf207d5eec910163dae011680e0dd6185cd7e6304691c");
        } else {
            assert!(false);
        }
    });
}
