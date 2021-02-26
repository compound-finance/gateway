use super::*;

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
                    sender: [254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56, 125, 95, 244, 112, 49],
                    recipient: [254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56, 125, 95, 244, 112, 49],
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
                    sender: [254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56, 125, 95, 244, 112, 49],
                    recipient: [254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56, 125, 95, 244, 112, 49],
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
                    sender: [254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107, 56, 125, 95, 244, 112, 49],
                    recipient: [81, 60, 31, 244, 53, 236, 206, 221, 15, 218, 94, 221, 42, 213, 229, 70, 31, 14, 135, 38],
                    amount: 5000000000000000,
                },
            }));
        } else {
            assert!(false);
        }
    });
}

// The next set of tests is checking block number from which OCW starts fetching events

#[test]
// OCW fetches events from the block number with the earliest `Pending` event
fn test_offchain_worker_min_in_pending() {
    use frame_support::traits::OffchainWorker;
    std::env::set_var("OPF_URL", TEST_OPF_URL);

    // `fromBlock` is important, core check of this test, equals hex(11938293)
    let events_call = testing::PendingRequest{
        method: "POST".into(),
        uri: "https://goerli-eth.compound.finance".into(),
        body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address": "0xbbde1662bC3ED16aA8C618c9833c801F3543B587", "fromBlock": "0xB602E5", "toBlock": "0xB60498"}],"id":1}"#.to_vec(),
        response: Some(testdata::json_responses::NO_EVENTS_RESPONSE.to_vec().clone()),
        headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
        sent: true,
        ..Default::default()
    };
    let calls = get_basic_calls(events_call);

    let (mut t, _pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);

    t.execute_with(|| {
        initialize_storage();

        // Set block number
        let block = 1;
        System::set_block_number(block);

        PendingEvents::insert(ChainLogId::Eth(11938293, 0), SignersSet::new());
        // Min block number is here, 11928293 == 0xB602E5
        PendingEvents::insert(ChainLogId::Eth(11928293, 0), SignersSet::new());
        PendingEvents::insert(ChainLogId::Eth(11928294, 0), SignersSet::new());

        // Execute offchain worker with no cached block number
        CashModule::offchain_worker(block);
    });
}

#[test]
// OCW fetches events from the next block number after the latest `Done` event
fn test_offchain_worker_max_in_done() {
    use frame_support::traits::OffchainWorker;
    std::env::set_var("OPF_URL", TEST_OPF_URL);

    // `fromBlock` is important, core check of this test, equals hex(11938293 + 1)
    let events_call = testing::PendingRequest{
        method: "POST".into(),
        uri: "https://goerli-eth.compound.finance".into(),
        body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address": "0xbbde1662bC3ED16aA8C618c9833c801F3543B587", "fromBlock": "0xB629F6", "toBlock": "0xB60498"}],"id":1}"#.to_vec(),
        response: Some(testdata::json_responses::NO_EVENTS_RESPONSE.to_vec().clone()),
        headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
        sent: true,
        ..Default::default()
    };
    let calls = get_basic_calls(events_call);

    let (mut t, _pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);

    t.execute_with(|| {
        initialize_storage();

        // Set block number
        let block = 1;
        System::set_block_number(block);

        // Max block number is here, 11938293 == 0xB629F5
        DoneEvents::insert(ChainLogId::Eth(11938293, 0), SignersSet::new());
        DoneEvents::insert(ChainLogId::Eth(11928293, 0), SignersSet::new());
        DoneEvents::insert(ChainLogId::Eth(11928294, 0), SignersSet::new());

        // Execute offchain worker with no cached block number
        CashModule::offchain_worker(block);
    });
}

#[test]
// OCW fetches events from the next block number after the latest `Failed` event
fn test_offchain_worker_max_in_failed() {
    use frame_support::traits::OffchainWorker;
    std::env::set_var("OPF_URL", TEST_OPF_URL);

    // `fromBlock` is important, core check of this test, equals hex(11938293 + 1)
    let events_call = testing::PendingRequest{
        method: "POST".into(),
        uri: "https://goerli-eth.compound.finance".into(),
        body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address": "0xbbde1662bC3ED16aA8C618c9833c801F3543B587", "fromBlock": "0xB629F6", "toBlock": "0xB60498"}],"id":1}"#.to_vec(),
        response: Some(testdata::json_responses::NO_EVENTS_RESPONSE.to_vec().clone()),
        headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
        sent: true,
        ..Default::default()
    };
    let calls = get_basic_calls(events_call);

    let (mut t, _pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);

    t.execute_with(|| {
        initialize_storage();

        // Set block number
        let block = 1;
        System::set_block_number(block);

        // Max block number is here, 11938293 == 0xB629F5
        FailedEvents::insert(ChainLogId::Eth(11928293, 0), Reason::None);
        FailedEvents::insert(ChainLogId::Eth(11928294, 0), Reason::None);
        FailedEvents::insert(ChainLogId::Eth(11938293, 0), Reason::None);

        // Execute offchain worker with no cached block number
        CashModule::offchain_worker(block);
    });
}

#[test]
// OCW fetches events from the next block number after the latest `Done` and `Failed` event
fn test_offchain_worker_max_in_done_and_failed() {
    use frame_support::traits::OffchainWorker;
    std::env::set_var("OPF_URL", TEST_OPF_URL);

    // `fromBlock` is important, core check of this test, equals hex(11938297 + 1)
    let events_call = testing::PendingRequest{
        method: "POST".into(),
        uri: "https://goerli-eth.compound.finance".into(),
        body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address": "0xbbde1662bC3ED16aA8C618c9833c801F3543B587", "fromBlock": "0xB629FA", "toBlock": "0xB60498"}],"id":1}"#.to_vec(),
        response: Some(testdata::json_responses::NO_EVENTS_RESPONSE.to_vec().clone()),
        headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
        sent: true,
        ..Default::default()
    };
    let calls = get_basic_calls(events_call);

    let (mut t, _pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);

    t.execute_with(|| {
        initialize_storage();

        // Set block number
        let block = 1;
        System::set_block_number(block);

        // Max block number is `Done` queue, 11938293 == 0xB629F5
        DoneEvents::insert(ChainLogId::Eth(11938293, 0), SignersSet::new());
        DoneEvents::insert(ChainLogId::Eth(11928293, 0), SignersSet::new());

        // Max block number is in `Failed` queue, 11938297 == 0xB629F9
        FailedEvents::insert(ChainLogId::Eth(11928295, 0), Reason::None);
        FailedEvents::insert(ChainLogId::Eth(11938297, 0), Reason::None);

        // Total max for `Failed` and `Done` queues will be 0xB629F9
        // Execute offchain worker with no cached block number
        CashModule::offchain_worker(block);
    });
}

#[test]
// OCW fetches events from the `earliest`
fn test_offchain_worker_no_events() {
    use frame_support::traits::OffchainWorker;
    std::env::set_var("OPF_URL", TEST_OPF_URL);

    let events_call = testing::PendingRequest{
        method: "POST".into(),
        uri: "https://goerli-eth.compound.finance".into(),
        body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address": "0xbbde1662bC3ED16aA8C618c9833c801F3543B587", "fromBlock": "earliest", "toBlock": "0xB60498"}],"id":1}"#.to_vec(),
        response: Some(testdata::json_responses::NO_EVENTS_RESPONSE.to_vec().clone()),
        headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
        sent: true,
        ..Default::default()
    };
    let calls = get_basic_calls(events_call);

    let (mut t, _pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);

    t.execute_with(|| {
        initialize_storage();

        // Set block number
        let block = 1;
        System::set_block_number(block);

        // Total max for `Failed` and `Done` queues will be 0xB629F9
        // Execute offchain worker with no cached block number
        CashModule::offchain_worker(block);
    });
}

fn get_basic_calls(events_call: testing::PendingRequest) -> Vec<testing::PendingRequest> {
    const LATEST_BLOCK_NUMBER_RESPONSE: &[u8; 79] = br#"{
        "jsonrpc": "2.0",
        "id": 1,
        "result": "0xB60498"
    }"#;

    // Find the latest eth block number
    let latest_block_call = testing::PendingRequest {
        method: "POST".into(),
        uri: "https://goerli-eth.compound.finance".into(),
        body: br#"{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}"#.to_vec(),
        response: Some(LATEST_BLOCK_NUMBER_RESPONSE.to_vec().clone()),
        headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
        sent: true,
        ..Default::default()
    };

    const PRICE_API_RESPONSE_NO_TEST_DATA: &str = r#"
    {
      "messages": [
      ],
      "prices": {
      },
      "signatures": [
      ],
      "timestamp": "1609340760"
    }
    "#;

    let price_call = testing::PendingRequest {
        method: "GET".into(),
        uri: TEST_OPF_URL.into(),
        body: vec![],
        response: Some(PRICE_API_RESPONSE_NO_TEST_DATA.to_owned().into_bytes()),
        headers: vec![],
        sent: true,
        ..Default::default()
    };

    return vec![latest_block_call, events_call, price_call];
}
