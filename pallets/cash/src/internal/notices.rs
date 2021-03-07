use frame_support::storage::{
    IterableStorageDoubleMap, IterableStorageMap, StorageDoubleMap, StorageMap,
};
use frame_system::offchain::SubmitTransaction;

use crate::{
    chains::{ChainAccount, ChainHash, ChainId, ChainSignature, ChainSignatureList},
    log,
    notices::{has_signer, EncodeNotice, NoticeId, NoticeState},
    reason::Reason,
    require, Call, Config, NoticeHashes, NoticeStates, Notices, Validators,
};

pub fn handle_notice_invoked<T: Config>(
    chain_id: ChainId,
    notice_id: NoticeId,
    notice_hash: ChainHash,
    _result: Vec<u8>,
) -> Result<(), Reason> {
    require!(
        NoticeHashes::get(notice_hash) == Some(notice_id),
        Reason::NoticeHashMismatch
    );
    Notices::take(chain_id, notice_id);
    NoticeStates::insert(chain_id, notice_id, NoticeState::Executed);
    Ok(())
}

fn process_notice_state<T: Config>(
    chain_id: ChainId,
    notice_id: NoticeId,
    notice_state: NoticeState,
) -> Result<bool, Reason> {
    match notice_state {
        NoticeState::Pending { signature_pairs } => {
            let signer = chain_id.signer_address()?;
            if !has_signer(&signature_pairs, signer) {
                let notice = Notices::get(chain_id, notice_id)
                    .ok_or(Reason::NoticeMissing(chain_id, notice_id))?;
                let signature: ChainSignature = notice.sign_notice()?; // NO_COV_FAIL: key already checked
                log!("Posting Signature for [{},{}]", notice_id.0, notice_id.1);

                let call = <Call<T>>::publish_signature(chain_id, notice_id, signature);
                SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
                    .map_err(|()| Reason::FailedToSubmitExtrinsic)?; // NO_COV_FAIL: extrinsic is valid

                Ok(true)
            } else {
                Ok(false)
            }
        }
        _ => Ok(false),
    }
}

pub fn process_notices<T: Config>(_block_number: T::BlockNumber) -> (usize, usize, Vec<Reason>) {
    NoticeStates::iter().fold((0, 0, vec![]), |(succ, skip, mut fail), (chain_id, notice_id, notice_state)| {
        match process_notice_state::<T>(chain_id, notice_id, notice_state) {
            Ok(true) => (succ + 1, skip, fail),
            Ok(false) => (succ, skip + 1, fail),
            Err(err) => {
                fail.push(err);
                (succ, skip, fail)
            }
        }
    })
}

pub fn publish_signature(
    chain_id: ChainId,
    notice_id: NoticeId,
    signature: ChainSignature,
) -> Result<(), Reason> {
    log!("Publishing Signature: [{},{}]", notice_id.0, notice_id.1);
    let state = NoticeStates::get(chain_id, notice_id);

    match state {
        NoticeState::Missing => Ok(()),

        NoticeState::Pending { signature_pairs } => {
            let notice = Notices::get(chain_id, notice_id)
                .ok_or(Reason::NoticeMissing(chain_id, notice_id))?;
            let signer: ChainAccount = signature.recover(&notice.encode_notice())?;
            let has_signer_v = has_signer(&signature_pairs, signer);

            if has_signer_v {
                return Ok(()); // Ignore for double-signs
            }

            // TODO: Can this be easier?
            let signature_pairs_next = match (signature_pairs, signer, signature) {
                (
                    ChainSignatureList::Eth(eth_signature_list),
                    ChainAccount::Eth(eth_account),
                    ChainSignature::Eth(eth_sig),
                ) => {
                    let mut eth_signature_list_mut = eth_signature_list.clone();
                    eth_signature_list_mut.push((eth_account, eth_sig));
                    Ok(ChainSignatureList::Eth(eth_signature_list_mut))
                }
                _ => Err(Reason::SignatureMismatch),
            }?;

            // Note: we currently iterate all potentially all validators to check validity
            if !Validators::iter().any(|(_, v)| ChainAccount::Eth(v.eth_address) == signer) {
                Err(Reason::UnknownValidator)?
            }

            NoticeStates::insert(
                chain_id,
                notice_id,
                NoticeState::Pending {
                    signature_pairs: signature_pairs_next,
                },
            );
            Ok(())
        }

        NoticeState::Executed => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::{Chain, ChainSignatureList, Ethereum},
        mock::*,
        notices::{ExtractionNotice, Notice},
        types::ValidatorKeys,
    };
    use gateway_crypto::CryptoError;
    use sp_core::crypto::AccountId32;

    /** `handle_notice_invoked` tests **/

    #[test]
    fn test_handle_notice_invoked_proper_notice() {
        new_test_ext().execute_with(|| {
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice_hash = ChainHash::Eth([1; 32]);
            let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });

            NoticeHashes::insert(notice_hash, notice_id);
            Notices::insert(chain_id, notice_id, notice);
            NoticeStates::insert(
                chain_id,
                notice_id,
                NoticeState::Pending {
                    signature_pairs: ChainSignatureList::Eth(vec![]),
                },
            );

            let result = handle_notice_invoked::<Test>(chain_id, notice_id, notice_hash, vec![]);

            assert_eq!(result, Ok(()));

            assert_eq!(NoticeHashes::get(notice_hash), Some(notice_id));
            assert_eq!(Notices::get(chain_id, notice_id), None);
            assert_eq!(
                NoticeStates::get(chain_id, notice_id),
                NoticeState::Executed
            );
        });
    }

    #[test]
    fn test_handle_notice_invoked_when_notice_missing() {
        new_test_ext().execute_with(|| {
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice_hash = ChainHash::Eth([1; 32]);

            NoticeHashes::insert(notice_hash, notice_id);

            let result = handle_notice_invoked::<Test>(chain_id, notice_id, notice_hash, vec![]);

            assert_eq!(result, Ok(()));

            assert_eq!(NoticeHashes::get(notice_hash), Some(notice_id));
            assert_eq!(Notices::get(chain_id, notice_id), None);
            assert_eq!(
                NoticeStates::get(chain_id, notice_id),
                NoticeState::Executed
            );
        });
    }

    #[test]
    fn test_handle_notice_invoked_mismatched_notice() {
        new_test_ext().execute_with(|| {
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice_hash = ChainHash::Eth([1; 32]);
            let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });

            NoticeHashes::insert(notice_hash, notice_id);
            Notices::insert(chain_id, notice_id, notice.clone());
            NoticeStates::insert(
                chain_id,
                notice_id,
                NoticeState::Pending {
                    signature_pairs: ChainSignatureList::Eth(vec![]),
                },
            );

            let result =
                handle_notice_invoked::<Test>(chain_id, NoticeId(66, 77), notice_hash, vec![]);

            assert_eq!(result, Err(Reason::NoticeHashMismatch));

            assert_eq!(NoticeHashes::get(notice_hash), Some(notice_id));
            assert_eq!(Notices::get(chain_id, notice_id), Some(notice));
            assert_eq!(
                NoticeStates::get(chain_id, notice_id),
                NoticeState::Pending {
                    signature_pairs: ChainSignatureList::Eth(vec![]),
                }
            );
        });
    }

    /** `process_notice_state` tests **/

    // Currently, the env vars set in other tests make this very difficult to test
    // since even if we don't set the key, some other test might. We should re-address
    // at a later point. That said, the code does work and this test is just for coverage.

    // #[test]
    // fn test_process_notice_state_missing_signer() {
    //     new_test_ext().execute_with(|| {
    //         let chain_id = ChainId::Eth;
    //         let notice_id = NoticeId(5, 6);
    //         let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
    //             id: NoticeId(80, 1),
    //             parent: [3u8; 32],
    //             asset: [1; 20],
    //             amount: 100,
    //             account: [2; 20],
    //         });
    //         let notice_state = NoticeState::pending(&notice);

    //         assert_eq!(
    //             process_notice_state::<Test>(chain_id, notice_id, notice_state),
    //             Err(Reason::KeyNotFound)
    //         );
    //     });
    // }

    #[test]
    fn test_process_notice_state_missing_notice() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });
            let notice_state = NoticeState::pending(&notice);

            assert_eq!(
                process_notice_state::<Test>(chain_id, notice_id, notice_state),
                Err(Reason::NoticeMissing(chain_id, notice_id))
            );
        });
    }

    #[test]
    fn test_process_notice_state_non_pending() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice_state = NoticeState::Executed {};

            assert_eq!(
                process_notice_state::<Test>(chain_id, notice_id, notice_state),
                Ok(false)
            );
        });
    }

    #[test]
    fn test_process_notice_state_already_signed() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let signer = <Ethereum as Chain>::signer_address().unwrap();
            let notice_state = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Eth(vec![(signer, [0u8; 65])]),
            };

            assert_eq!(
                process_notice_state::<Test>(chain_id, notice_id, notice_state),
                Ok(false)
            );
        });
    }

    #[test]
    fn test_process_notice_state_valid() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });
            let notice_state = NoticeState::pending(&notice);
            Notices::insert(chain_id, notice_id, notice);

            assert_eq!(
                process_notice_state::<Test>(chain_id, notice_id, notice_state),
                Ok(true)
            );
        });
    }

    /** `process_notices` tests **/

    #[test]
    fn test_process_notices() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
            let chain_id = ChainId::Eth;
            let notice_id_1 = NoticeId(5, 6);
            let notice_1 = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });

            // Proper pending
            let notice_state_1 = NoticeState::pending(&notice_1);
            Notices::insert(chain_id, notice_id_1, notice_1);
            NoticeStates::insert(chain_id, notice_id_1, notice_state_1);

            // Executed
            let notice_id_2 = NoticeId(5, 7);
            let notice_state_2 = NoticeState::Executed {};
            NoticeStates::insert(chain_id, notice_id_2, notice_state_2);

            // Missing
            let notice_id_3 = NoticeId(5, 8);
            let notice_state_3 = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Eth(vec![]),
            };
            NoticeStates::insert(chain_id, notice_id_3, notice_state_3);

            assert_eq!(
                process_notices::<Test>(0u64.into()),
                (1, 1, vec![Reason::NoticeMissing(chain_id, notice_id_3)])
            );
        });
    }

    /** `publish_signature` tests **/

    #[test]
    fn test_publish_signature_missing() {
        new_test_ext().execute_with(|| {
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice_state = NoticeState::Missing {};
            let signature = ChainSignature::Eth([1u8; 65]);
            NoticeStates::insert(chain_id, notice_id, notice_state);

            assert_eq!(publish_signature(chain_id, notice_id, signature), Ok(()));
        });
    }

    #[test]
    fn test_publish_signature_executed() {
        new_test_ext().execute_with(|| {
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice_state = NoticeState::Executed {};
            let signature = ChainSignature::Eth([1u8; 65]);
            NoticeStates::insert(chain_id, notice_id, notice_state);

            assert_eq!(publish_signature(chain_id, notice_id, signature), Ok(()));
        });
    }

    #[test]
    fn test_publish_signature_pending_valid() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
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

            assert_eq!(publish_signature(chain_id, notice_id, signature), Ok(()));

            assert_eq!(
                NoticeStates::get(chain_id, notice_id),
                expected_notice_state
            );
        });
    }

    #[test]
    fn test_publish_signature_pending_and_missing() {
        new_test_ext().execute_with(|| {
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });
            let notice_state = NoticeState::pending(&notice);
            let signature = ChainSignature::Eth([1u8; 65]);
            NoticeStates::insert(chain_id, notice_id, notice_state);

            assert_eq!(
                publish_signature(chain_id, notice_id, signature),
                Err(Reason::NoticeMissing(chain_id, notice_id))
            );
        });
    }

    #[test]
    fn test_publish_signature_pending_already_signed() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
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
            let signer = <Ethereum as Chain>::signer_address().unwrap();
            let notice_state = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Eth(vec![(signer, eth_signature)]),
            };
            NoticeStates::insert(chain_id, notice_id, notice_state);
            Notices::insert(chain_id, notice_id, notice);

            assert_eq!(publish_signature(chain_id, notice_id, signature), Ok(()));
        });
    }

    #[test]
    fn test_publish_signature_signature_mismatch() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
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
                _ => panic!("invalid signature"),
            };
            let signer = <Ethereum as Chain>::signer_address().unwrap();
            let notice_state = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Comp(vec![(signer, eth_signature)]),
            };
            NoticeStates::insert(chain_id, notice_id, notice_state);
            Notices::insert(chain_id, notice_id, notice);

            assert_eq!(
                publish_signature(chain_id, notice_id, signature),
                Err(Reason::SignatureMismatch)
            );
        });
    }

    #[test]
    fn test_publish_signature_pending_unknown_validator() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });
            let signature = ChainSignature::Eth([1u8; 65]);
            let notice_state = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Eth(vec![([1u8; 20], [1u8; 65])]),
            };
            NoticeStates::insert(chain_id, notice_id, notice_state);
            Notices::insert(chain_id, notice_id, notice);

            assert_eq!(
                publish_signature(chain_id, notice_id, signature),
                Err(Reason::UnknownValidator)
            );
        });
    }

    #[test]
    fn test_publish_signature_pending_invalid_signature() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });
            let mut signature = notice.sign_notice().unwrap();
            let eth_signature = match signature {
                ChainSignature::Eth(ref mut a) => {
                    a[64] = 2;
                    a
                }
                _ => panic!("invalid signature"),
            };
            let signer = <Ethereum as Chain>::signer_address().unwrap();
            let notice_state = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Eth(vec![(signer, *eth_signature)]),
            };
            NoticeStates::insert(chain_id, notice_id, notice_state);
            Notices::insert(chain_id, notice_id, notice);

            assert_eq!(
                publish_signature(chain_id, notice_id, signature),
                Err(Reason::CryptoError(CryptoError::RecoverError))
            );
        });
    }
}
