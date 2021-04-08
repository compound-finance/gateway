#![cfg_attr(not(feature = "std"), no_std)]
use super::{Module as Cash, *};
use codec::Encode;
use frame_benchmarking::benchmarks;
pub use frame_support::{assert_err, assert_ok};
use frame_system::RawOrigin;
use sp_core::crypto::AccountId32;
use sp_std::prelude::*;

use crate::{
    chains::{Chain, ChainSignatureList, Ethereum},
    notices::{ExtractionNotice, Notice},
    rates::APR,
    types::ValidatorKeys,
};

benchmarks! {
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

    set_yield_next {
        assert_eq!(CashYieldNext::get(), None);
    }: {
        assert_eq!(Cash::<T>::set_yield_next(RawOrigin::Root.into(), APR(100).into(), 86400500), Ok(()));
    }

    receive_chain_blocks {
        let substrate_id = AccountId32::new([12u8; 32]);
        let eth_address = <Ethereum as Chain>::signer_address().unwrap();
        Validators::insert(
            substrate_id.clone(),
            ValidatorKeys {
                substrate_id,
                eth_address,
            },
        );
        let blocks = ChainBlocks::Eth(vec![]);
        let signature = <Ethereum as Chain>::sign_message(&blocks.encode()).unwrap();

    }: {
        assert_ok!(Cash::<T>::receive_chain_blocks(
            RawOrigin::None.into(),
            blocks,
            signature
        ));
    }

}

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use crate::tests::{
        initialize_storage,
        mock::{new_test_ext, Test},
    };

    #[test]
    fn test_benchmarks() {
        new_test_ext().execute_with(|| {
            initialize_storage();
            assert_ok!(test_benchmark_receive_chain_blocks::<Test>());
            assert_ok!(test_benchmark_publish_signature::<Test>());
            assert_ok!(test_benchmark_set_yield_next::<Test>());
        });
    }
}
