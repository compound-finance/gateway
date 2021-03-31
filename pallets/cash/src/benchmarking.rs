#![cfg_attr(not(feature = "std"), no_std)]
use frame_system::RawOrigin;
use frame_benchmarking::benchmarks;
pub use frame_support::{assert_err, assert_ok};
use super::{*, Module as Cash};
use sp_std::prelude::*;

use crate::{
  chains::{Chain, ChainSignatureList, Ethereum},
  notices::{ExtractionNotice, Notice},
  types::ValidatorKeys,
  rates::APR
};

benchmarks!{
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

  // test fails, presumably bc timestamp not set correctly
  set_yield_next{
    assert_eq!(CashYieldNext::get(), None);
  }: {
    assert_eq!(Cash::<T>::set_yield_next(RawOrigin::None.into(), APR(100).into(), 86400500), Ok(()));
  }

  receive_event {
    // Set validator signing key
    std::env::set_var(
        "ETH_KEY",
        "6bc5ea78f041146e38233f5bc29c703c1cec8eaaa2214353ee8adf7fc598f23d",
    );

    // Dispatch a signed extrinsic.
    let event_id = ChainLogId::Eth(3858223, 0);
    let event = ChainLogEvent::Eth(ethereum_client::EthereumLogEvent {
        block_hash: [3; 32],
        block_number: 3858223,
        transaction_index: 0,
        log_index: 0,
        event: ethereum_client::EthereumEvent::Lock {
            asset: [1; 20],
            sender: [3; 20],
            chain: String::from("ETH"),
            recipient: [2; 32],
            amount: 10,
        },
    });
    // doesnt compile
    let payload = event.encode();
    let signature = <Ethereum as Chain>::sign_message(&payload).unwrap(); // Sign with our "shared" private key for now XXX

  }: {
    assert_ok!(Cash::receive_event(
        RawOrigin::None.into(),
        event_id,
        event,
        signature
    ));
  }

}

#[cfg(test)]
mod benchmark_tests {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::assert_ok;

  fn test_benchmarks() {
    new_test_ext().execute_with(|| {
      tests::initialize_storage();
      assert_ok!(test_benchmark_publish_signature::<Test>());
      assert_ok!(test_benchmark_set_yield_next::<Test>());
      assert_ok!(test_benchmark_receive_event::<Test>());
    });
  }
}