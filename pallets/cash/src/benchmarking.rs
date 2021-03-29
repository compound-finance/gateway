#![cfg_attr(not(feature = "std"), no_std)]
use frame_system::RawOrigin;
use frame_benchmarking::benchmarks;
use super::{*, Module as Cash};
use sp_std::prelude::*;

use crate::{
  chains::{Chain, ChainSignatureList, Ethereum},
  notices::{ExtractionNotice, Notice},
  types::ValidatorKeys,
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

}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::assert_ok;

  fn test_benchmarks() {
    new_test_ext().execute_with(|| {
      assert_ok!(cull_notices::<Test>());
    });
  }
}