use super::test;
use super::*;

use frame_support::traits::OffchainWorker;

#[test]
fn test_offchain_worker() {
    // XXX this creates problems for test parallelization...
    runtime_interfaces::config_interface::set(runtime_interfaces::new_config(
        "0xbbde1662bC3ED16aA8C618c9833c801F3543B587".into(),
        runtime_interfaces::NULL_ETH_BLOCK,
    ));

    let calls = vec![
        testing::PendingRequest {
            method: "POST".into(),
            uri: "https://ropsten-eth.compound.finance".to_string(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: br#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x1",true],"id":1}"#.to_vec(),
            response: Some(tests::testdata::json_responses::GET_BLOCK_BY_NUMBER_1.to_vec()),
            sent: true,
            ..Default::default()
        },
        testing::PendingRequest {
            method: "POST".into(),
            uri: "https://ropsten-eth.compound.finance".to_string(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address":"0xbbde1662bC3ED16aA8C618c9833c801F3543B587","fromBlock":"0x1","toBlock":"0x1"}],"id":1}"#.to_vec(),
            response: Some(testdata::json_responses::GET_LOGS_1.to_vec()),
            sent: true,
            ..Default::default()
        },
        testing::PendingRequest {
            method: "POST".into(),
            uri: "https://ropsten-eth.compound.finance".to_string(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: br#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x2",true],"id":1}"#.to_vec(),
            response: Some(tests::testdata::json_responses::GET_BLOCK_BY_NUMBER_2.to_vec()),
            sent: true,
            ..Default::default()
        },
        testing::PendingRequest {
            method: "POST".into(),
            uri: "https://ropsten-eth.compound.finance".to_string(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: br#"{"jsonrpc":"2.0","method":"eth_getLogs","params":[{"address":"0xbbde1662bC3ED16aA8C618c9833c801F3543B587","fromBlock":"0x2","toBlock":"0x2"}],"id":1}"#.to_vec(),
            response: Some(testdata::json_responses::GET_LOGS_2.to_vec()),
            sent: true,
            ..Default::default()
        },
        testing::PendingRequest {
            method: "POST".into(),
            uri: "https://ropsten-eth.compound.finance".to_string(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: br#"{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x3",true],"id":1}"#.to_vec(),
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
            assert_eq!(blocks.len(), 1);
            match blocks {
                ChainBlocks::Eth(blocks) => {
                    let block = &blocks[0];
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
            }

            assert_eq!(PendingChainBlocks::get(ChainId::Eth), vec![]); // XXX how to execute extrinsic?
        } else {
            assert!(false);
        }
    });
}
