use super::test;
use super::*;

use frame_support::traits::OffchainWorker;

#[test]
fn test_offchain_worker() {
    let calls = vec![
        testing::PendingRequest {
            method: "POST".into(),
            uri: "https://ropsten-eth.compound.finance".to_string(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: br#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x2",false],"id":1}"#.to_vec(),
            response: Some(tests::testdata::json_responses::GET_BLOCK_BY_NUMBER_2.to_vec()),
            sent: true,
            ..Default::default()
        },
        testing::PendingRequest {
            method: "POST".into(),
            uri: "https://ropsten-eth.compound.finance".to_string(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address":"0x7777777777777777777777777777777777777777","blockHash":"0x50314c1c6837e15e60c5b6732f092118dd25e3ec681f5e089b3a9ad2374e5a8a"}],"id":1}"#.to_vec(),
            response: Some(testdata::json_responses::GET_LOGS_2.to_vec()),
            sent: true,
            ..Default::default()
        },
        testing::PendingRequest {
            method: "POST".into(),
            uri: "https://ropsten-eth.compound.finance".to_string(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: br#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x3",false],"id":1}"#.to_vec(),
            response: Some(tests::testdata::json_responses::GET_BLOCK_BY_NUMBER_3.to_vec()),
            sent: true,
            ..Default::default()
        },
        testing::PendingRequest {
            method: "POST".into(),
            uri: "https://ropsten-eth.compound.finance".to_string(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address":"0x7777777777777777777777777777777777777777","blockHash":"0x72314c1c6837e15e60c5b6732f092118dd25e3ec681f5e089b3a9ad2374e5a8a"}],"id":1}"#.to_vec(),
            response: Some(testdata::json_responses::GET_LOGS_3.to_vec()),
            sent: true,
            ..Default::default()
        },
        testing::PendingRequest {
            method: "POST".into(),
            uri: "https://ropsten-eth.compound.finance".to_string(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: br#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x4",false],"id":1}"#.to_vec(),
            response: Some(tests::testdata::json_responses::NO_RESULT.to_vec()),
            sent: true,
            ..Default::default()
        },
    ];

    let (mut t, pool_state, _offchain_state) = new_test_ext_with_http_calls(calls);

    t.execute_with(|| {
        initialize_storage();

        let block_num = 1;
        System::set_block_number(block_num);
        CashModule::offchain_worker(block_num);

        assert_eq!(pool_state.read().transactions.len(), 1);

        let tx1 = pool_state.write().transactions.pop().unwrap();
        let ex1: Extrinsic = Decode::decode(&mut &*tx1).unwrap();

        assert_eq!(ex1.signature, None);

        if let mock::Call::Cash(crate::Call::receive_chain_blocks(blocks, _signature)) = ex1.call {
            assert_eq!(blocks.chain_id(), ChainId::Eth);
            assert_eq!(blocks.len(), 2);
            match blocks {
                ChainBlocks::Eth(blocks) => {
                    let block = &blocks[1];
                    assert_eq!(block.events.len(), 3);

                    let event = &block.events[1];
                    assert_eq!(
                        event.clone(),
                        ethereum_client::EthereumEvent::Lock {
                            asset: [
                                216, 123, 167, 165, 11, 46, 126, 102, 15, 103, 138, 137, 94, 75,
                                114, 231, 203, 76, 205, 156
                            ],
                            sender: [
                                254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107,
                                56, 125, 95, 244, 112, 49
                            ],
                            chain: String::from("ETH"),
                            recipient: [
                                254, 177, 234, 39, 248, 136, 195, 132, 241, 176, 220, 20, 253, 107,
                                56, 125, 95, 244, 112, 49, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
                            ],
                            amount: 1000000000000000000,
                        }
                    )
                }
                _ => unreachable!(),
            }

            assert_eq!(PendingChainBlocks::get(ChainId::Eth), vec![]); // XXX how to execute extrinsic?
        } else {
            assert!(false);
        }
    });
}
