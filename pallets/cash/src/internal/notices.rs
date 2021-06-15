use crate::{
    chains::{ChainAccount, ChainAsset, ChainHash, ChainId, ChainSignature},
    core::recover_validator,
    log,
    notices::{
        CashExtractionNotice, ChangeAuthorityNotice, EncodeNotice, ExtractionNotice,
        FutureYieldNotice, Notice, NoticeId, NoticeState, SetSupplyCapNotice,
    },
    require,
    types::{
        AssetAmount, AssetQuantity, CashIndex, CashPrincipalAmount, Reason, Timestamp,
        ValidatorKeys, APR,
    },
    AccountNotices, Call, Config, Event, LatestNotice, Module, NoticeHashes, NoticeHolds,
    NoticeStates, Notices,
};
use frame_support::storage::{IterableStorageDoubleMap, StorageDoubleMap, StorageMap};
use frame_system::offchain::SubmitTransaction;

pub fn dispatch_extraction_notice<T: Config>(
    asset: ChainAsset,
    recipient: ChainAccount,
    amount: AssetQuantity,
) {
    dispatch_notice::<T>(
        recipient.chain_id(),
        Some(recipient),
        false,
        &|notice_id, parent_hash| {
            Notice::ExtractionNotice(match (asset, recipient, parent_hash) {
                (
                    ChainAsset::Eth(eth_asset),
                    ChainAccount::Eth(eth_account),
                    ChainHash::Eth(eth_parent_hash),
                ) => ExtractionNotice::Eth {
                    id: notice_id,
                    parent: eth_parent_hash,
                    asset: eth_asset,
                    account: eth_account,
                    amount: amount.value,
                },

                _ => panic!("XXX not implemented"), // generate these w/ macros?
            })
        },
    )
}

pub fn dispatch_cash_extraction_notice<T: Config>(
    recipient: ChainAccount,
    principal: CashPrincipalAmount,
) {
    dispatch_notice::<T>(
        recipient.chain_id(),
        Some(recipient),
        false,
        &|notice_id, parent_hash| {
            Notice::CashExtractionNotice(match (recipient, parent_hash) {
                (ChainAccount::Eth(eth_account), ChainHash::Eth(eth_parent_hash)) => {
                    CashExtractionNotice::Eth {
                        id: notice_id,
                        parent: eth_parent_hash,
                        account: eth_account,
                        principal: principal.0,
                    }
                }

                _ => panic!("XXX not implemented"), // generate these w/ macros?
            })
        },
    )
}

pub fn dispatch_supply_cap_notice<T: Config>(chain_asset: ChainAsset, cap: AssetAmount) {
    dispatch_notice::<T>(
        chain_asset.chain_id(),
        None,
        true,
        &|notice_id, parent_hash| {
            Notice::SetSupplyCapNotice(match (chain_asset, parent_hash) {
                (ChainAsset::Eth(eth_asset), ChainHash::Eth(eth_parent_hash)) => {
                    SetSupplyCapNotice::Eth {
                        id: notice_id,
                        parent: eth_parent_hash,
                        asset: eth_asset,
                        cap,
                    }
                }

                _ => panic!("XXX not implemented"), // generate these w/ macros?
            })
        },
    )
}

pub fn dispatch_future_yield_notice<T: Config>(
    next_yield: APR,
    next_yield_index: CashIndex,
    next_yield_start: Timestamp,
) {
    // XXX for each chain id
    let chain_id = ChainId::Eth;
    dispatch_notice::<T>(chain_id, None, true, &|notice_id, parent_hash| {
        Notice::FutureYieldNotice(match parent_hash {
            ChainHash::Eth(eth_parent_hash) => FutureYieldNotice::Eth {
                id: notice_id,
                parent: eth_parent_hash,
                next_cash_yield: next_yield.0,
                next_cash_index: next_yield_index.0,
                next_cash_yield_start: next_yield_start,
            },

            _ => panic!("XXX not implemented"), // generate these w/ macros?
        })
    })
}

pub fn dispatch_change_authority_notice<T: Config>(validators: Vec<ValidatorKeys>) {
    // XXX for each chain id
    let chain_id = ChainId::Eth;
    dispatch_notice::<T>(chain_id, None, true, &|notice_id, parent_hash| {
        Notice::ChangeAuthorityNotice(match parent_hash {
            ChainHash::Eth(eth_parent_hash) => ChangeAuthorityNotice::Eth {
                id: notice_id,
                parent: eth_parent_hash,
                new_authorities: validators.iter().map(|x| x.eth_address).collect::<Vec<_>>(),
            },

            _ => panic!("XXX not implemented"), // generate these w/ macros?
        })
    })
}

/// Add a notice to the queue and all the secondary indices.
/// Called from effects phase and thus may not fail.
fn dispatch_notice<T: Config>(
    chain_id: ChainId,
    maybe_recipient: Option<ChainAccount>,
    should_increment_era: bool,
    notice_fn: &dyn Fn(NoticeId, ChainHash) -> Notice,
) {
    let (latest_notice_id, parent_hash) =
        LatestNotice::get(chain_id).unwrap_or((NoticeId(0, 0), chain_id.zero_hash()));

    let notice_id = if should_increment_era {
        latest_notice_id.seq_era()
    } else {
        latest_notice_id.seq()
    };

    // Add to notices, notice states, track the latest notice and index by account
    let notice = notice_fn(notice_id, parent_hash);
    let notice_hash = notice.hash();
    Notices::insert(chain_id, notice_id, &notice);
    NoticeStates::insert(chain_id, notice_id, NoticeState::pending(&notice));
    LatestNotice::insert(chain_id, (notice_id, notice_hash));
    NoticeHashes::insert(notice_hash, notice_id);
    if let Some(recipient) = maybe_recipient {
        AccountNotices::append(recipient, notice_id);
    }

    if let Notice::ChangeAuthorityNotice(_) = &notice {
        NoticeHolds::insert(chain_id, notice_id);
    }

    // Deposit Notice Event
    let encoded_notice = notice.encode_notice();
    Module::<T>::deposit_event(Event::Notice(notice_id, notice, encoded_notice));
}

pub fn handle_notice_invoked<T: Config>(
    chain_id: ChainId,
    notice_id: NoticeId,
    notice_hash: ChainHash,
    _result: Vec<u8>,
) -> Result<(), Reason> {
    require!(
        NoticeHashes::get(notice_hash) == Some(notice_id),
        Reason::HashMismatch
    );
    Notices::take(chain_id, notice_id);
    if let Some(notice_hold_id) = NoticeHolds::get(chain_id) {
        if notice_hold_id == notice_id {
            log!("Removing notice hold as executed");
            NoticeHolds::take(chain_id);
        }
    }
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
            if !signature_pairs.has_signer(signer) {
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

pub fn publish_signature<T: Config>(
    chain_id: ChainId,
    notice_id: NoticeId,
    signature: ChainSignature,
) -> Result<(), Reason> {
    log!("Publishing Signature: [{},{}]", notice_id.0, notice_id.1);

    match NoticeStates::get(chain_id, notice_id) {
        NoticeState::Missing => Ok(()),

        NoticeState::Pending {
            mut signature_pairs,
        } => {
            let notice = Notices::get(chain_id, notice_id)
                .ok_or(Reason::NoticeMissing(chain_id, notice_id))?;
            let validator = recover_validator::<T>(&notice.encode_notice(), signature)?;

            if signature_pairs.has_validator_signature(signature.chain_id(), &validator) {
                return Ok(());
            }

            signature_pairs.add_validator_signature(&signature, &validator)?;

            NoticeStates::insert(
                chain_id,
                notice_id,
                NoticeState::Pending { signature_pairs },
            );

            Ok(())
        }

        NoticeState::Executed => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    /** `handle_notice_invoked` tests **/

    #[test]
    fn test_handle_notice_invoked_proper_notice() {
        new_test_ext().execute_with(|| {
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice_hold_id = NoticeId(4, 6);
            let notice_hash = ChainHash::Eth([1; 32]);
            let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });

            NoticeHolds::insert(chain_id, notice_hold_id);
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
            assert_eq!(NoticeHolds::get(chain_id), Some(notice_hold_id));
        });
    }

    #[test]
    fn test_handle_notice_invoked_proper_notice_removes_notice_hold() {
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

            NoticeHolds::insert(chain_id, notice_id);
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
            assert_eq!(NoticeHolds::get(chain_id), None);
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

            assert_eq!(result, Err(Reason::HashMismatch));

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

            assert_eq!(
                publish_signature::<Test>(chain_id, notice_id, signature),
                Ok(())
            );
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

            assert_eq!(
                publish_signature::<Test>(chain_id, notice_id, signature),
                Ok(())
            );
        });
    }

    #[test]
    fn test_publish_signature_pending_valid() {
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
            let signature = notice.sign_notice().unwrap();
            let signer = <Ethereum as Chain>::signer_address().unwrap();
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
            Validators::insert(
                substrate_id.clone(),
                ValidatorKeys {
                    substrate_id,
                    eth_address: signer,
                },
            );

            let expected_notice_state = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Eth(vec![(signer, eth_signature)]),
            };

            assert_eq!(
                publish_signature::<Test>(chain_id, notice_id, signature),
                Ok(())
            );

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
                publish_signature::<Test>(chain_id, notice_id, signature),
                Err(Reason::NoticeMissing(chain_id, notice_id))
            );
        });
    }

    #[test]
    fn test_publish_signature_pending_already_signed() {
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

            let substrate_id = AccountId32::new([0u8; 32]);
            Validators::insert(
                substrate_id.clone(),
                ValidatorKeys {
                    substrate_id,
                    eth_address: signer,
                },
            );

            assert_eq!(
                publish_signature::<Test>(chain_id, notice_id, signature),
                Ok(())
            );
        });
    }

    #[test]
    fn test_publish_signature_signature_mismatch() {
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
            let signer = <Ethereum as Chain>::signer_address().unwrap();
            let signature = notice.sign_notice().unwrap();
            let eth_signature = match signature {
                ChainSignature::Eth(a) => a,
                _ => panic!("invalid signature"),
            };
            let notice_state = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Dot(vec![([0; 32], eth_signature)]),
            };
            NoticeStates::insert(chain_id, notice_id, notice_state);
            Notices::insert(chain_id, notice_id, notice);

            let substrate_id = AccountId32::new([0u8; 32]);

            Validators::insert(
                substrate_id.clone(),
                ValidatorKeys {
                    substrate_id,
                    eth_address: signer,
                },
            );

            assert_eq!(
                publish_signature::<Test>(chain_id, notice_id, signature),
                Err(Reason::SignatureMismatch)
            );
        });
    }

    #[test]
    fn test_publish_signature_pending_unknown_validator() {
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
            let signature = ChainSignature::Eth([1u8; 65]);
            let notice_state = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Eth(vec![([1u8; 20], [1u8; 65])]),
            };
            NoticeStates::insert(chain_id, notice_id, notice_state);
            Notices::insert(chain_id, notice_id, notice);

            assert_eq!(
                publish_signature::<Test>(chain_id, notice_id, signature),
                Err(Reason::UnknownValidator)
            );
        });
    }

    #[test]
    fn test_publish_signature_pending_invalid_signature() {
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
                publish_signature::<Test>(chain_id, notice_id, signature),
                Err(Reason::CryptoError(
                    gateway_crypto::CryptoError::RecoverError
                ))
            );
        });
    }
}
