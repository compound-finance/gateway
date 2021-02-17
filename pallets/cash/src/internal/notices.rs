use crate::{
    chains::{ChainHash, ChainId},
    notices::{NoticeId, NoticeState},
    reason::Reason,
    require, Config, NoticeHashes, NoticeStates, Notices,
};
use frame_support::storage::{StorageDoubleMap, StorageMap};

pub fn handle_notice_invoked<T: Config>(
    chain_id: ChainId,
    notice_id: NoticeId,
    notice_hash: ChainHash,
    _result: Vec<u8>,
) -> Result<(), Reason> {
    require!(
        NoticeHashes::get(notice_hash) == Some(notice_id),
        Reason::MismatchedNoticeHash
    );
    Notices::take(chain_id, notice_id);
    NoticeStates::insert(chain_id, notice_id, NoticeState::Executed);
    Ok(())
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

            assert_eq!(result, Err(Reason::MismatchedNoticeHash));

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
