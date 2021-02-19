use frame_support::storage::{IterableStorageDoubleMap, StorageDoubleMap, StorageMap};
use frame_system::offchain::SubmitTransaction;

use crate::{
    chains::{ChainAccount, ChainHash, ChainId, ChainSignature, ChainSignatureList},
    log,
    notices::{has_signer, EncodeNotice, NoticeId, NoticeState},
    reason::Reason,
    require, Call, Config, NoticeHashes, NoticeStates, Notices,
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

pub fn process_notices<T: Config>(_block_number: T::BlockNumber) -> Result<(), Reason> {
    // TODO: Do we want to return a failure here in any case, or collect them, etc?
    for (chain_id, notice_id, notice_state) in NoticeStates::iter() {
        match notice_state {
            NoticeState::Pending { signature_pairs } => {
                let signer = chain_id.signer_address()?;

                if !has_signer(&signature_pairs, signer) {
                    // find parent
                    // id = notice.gen_id(parent)

                    // XXX
                    // submit onchain call for aggregating the price
                    let notice = Notices::get(chain_id, notice_id)
                        .ok_or(Reason::NoticeMissing(chain_id, notice_id))?;
                    let signature: ChainSignature = notice.sign_notice()?;
                    log!("Posting Signature for [{},{}]", notice_id.0, notice_id.1);
                    let call = <Call<T>>::publish_signature(chain_id, notice_id, signature);

                    // TODO: Do we want to short-circuit on an error here?
                    let _res =
                        SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
                            .map_err(|()| Reason::FailedToSubmitExtrinsic);
                }

                ()
            }
            _ => (),
        }
    }
    Ok(())
}

pub fn publish_signature(
    chain_id: ChainId,
    notice_id: NoticeId,
    signature: ChainSignature,
) -> Result<(), Reason> {
    log!("Publishing Signature: [{},{}]", notice_id.0, notice_id.1);
    let state = <NoticeStates>::get(chain_id, notice_id);

    match state {
        NoticeState::Missing => Ok(()),

        NoticeState::Pending { signature_pairs } => {
            let notice = Notices::get(chain_id, notice_id)
                .ok_or(Reason::NoticeMissing(chain_id, notice_id))?;
            let signer: ChainAccount = signature.recover(&notice.encode_notice())?; // XXX
            let has_signer_v = has_signer(&signature_pairs, signer);

            require!(!has_signer_v, Reason::NoticeAlreadySigned);

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

            // require(signer == known validator); // XXX

            // XXX sets?
            // let signers_new = {signer | signers};
            // let signatures_new = {signature | signatures };

            // if len(signers_new & Validators) > 2/3 * len(Validators) {
            // NoticeQueue::insert(notice_id, NoticeState::Done);
            // Self::deposit_event(Event::SignedNotice(ChainId::Eth, notice_id, notice.encode_notice(), signatures)); // XXX generic
            // Ok(())
            // } else {
            NoticeStates::insert(
                chain_id,
                notice_id,
                NoticeState::Pending {
                    signature_pairs: signature_pairs_next,
                },
            );
            Ok(())
            // }
        }

        NoticeState::Executed => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::ChainSignatureList,
        mock::*,
        notices::{ExtractionNotice, Notice},
    };

    #[test]
    fn test_handle_proper_notice() {
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
    fn test_when_notice_missing() {
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
    fn test_handle_mismatched_notice() {
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
}
