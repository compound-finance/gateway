use crate::{
    chains::{ChainHash, ChainId},
    core::{dispatch_notice_internal, get_now},
    notices::{FutureYieldNotice, Notice},
    params::MIN_NEXT_SYNC_TIME,
    rates::APR,
    reason::Reason,
    require,
    types::{CashIndex, Timestamp},
    CashYield, CashYieldNext, Config, GlobalCashIndex,
};
use codec::{Decode, Encode};
use frame_support::storage::StorageValue;
use our_std::Debuggable;

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Debuggable)]
pub enum SetYieldNextError {
    TimestampTooSoonToNow,
    TimestampTooSoonToNext,
}

fn get_cash_yield_index_after<T: Config>(change_in_time: Timestamp) -> Result<CashIndex, Reason> {
    let cash_yield = CashYield::get();
    let cash_index_old = GlobalCashIndex::get();
    let increment = cash_yield.compound(change_in_time)?;
    cash_index_old
        .increment(increment)
        .map_err(Reason::MathError)
}

pub fn set_yield_next<T: Config>(next_apr: APR, next_apr_start: Timestamp) -> Result<(), Reason> {
    let now = get_now::<T>();
    let change_in_time = next_apr_start
        .checked_sub(now)
        .ok_or(Reason::TimeTravelNotAllowed)?;

    // TODO: Maybe check if `now` is zero / default?

    // Read Require CashYieldNext.NextAPRStartNow+ParamsMinNextSyncTime
    if let Some((_, next_start)) = CashYieldNext::get() {
        require!(
            next_start >= now + MIN_NEXT_SYNC_TIME,
            Reason::SetYieldNextError(SetYieldNextError::TimestampTooSoonToNext)
        );
    }

    // Note: Any already pending change cannot be canceled unless the min sync time is also met for the cancel.
    // Read Require NextAPRStartNow+ParamsMinNextSyncTime
    require!(
        next_apr_start >= now + MIN_NEXT_SYNC_TIME,
        Reason::SetYieldNextError(SetYieldNextError::TimestampTooSoonToNow)
    );

    // Read NextYieldIndex=GetCashYieldIndexAt(NextAPRStart)
    let next_yield_index: CashIndex = get_cash_yield_index_after::<T>(change_in_time)?;

    // Set CashYieldNext=(NextAPR, NextAPRStart)
    CashYieldNext::put((next_apr, next_apr_start));

    // For ChainChains:
    // Add FutureYieldNotice(NextAPR, NextAPRStart, NextYieldIndex) to NoticeQueueChain
    dispatch_notice_internal::<T>(ChainId::Eth, None, true, &|notice_id, parent_hash| {
        Ok(Notice::FutureYieldNotice(match parent_hash {
            ChainHash::Eth(eth_parent_hash) => FutureYieldNotice::Eth {
                id: notice_id,
                parent: eth_parent_hash,
                next_cash_yield: next_apr.0,
                next_cash_index: next_yield_index.0,
                next_cash_yield_start: next_apr_start,
            },
            _ => panic!("not supported"), // TODO: Panic?
        }))
    })?;

    // XXX Events
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::ChainSignatureList,
        mock::*,
        notices::{NoticeId, NoticeState},
        LatestNotice, NoticeStates, Notices,
    };
    use frame_support::storage::{
        IterableStorageDoubleMap, StorageDoubleMap, StorageMap, StorageValue,
    };

    #[test]
    fn test_too_soon_to_now() {
        new_test_ext().execute_with(|| {
            <pallet_timestamp::Module<Test>>::set_timestamp(500);
            assert_eq!(
                set_yield_next::<Test>(APR(100), 501),
                Err(Reason::SetYieldNextError(
                    SetYieldNextError::TimestampTooSoonToNow
                ))
            );
        });
    }

    #[test]
    fn test_too_soon_to_next() {
        new_test_ext().execute_with(|| {
            CashYieldNext::put((APR(100), 500));
            assert_eq!(
                set_yield_next::<Test>(APR(100), 501),
                Err(Reason::SetYieldNextError(
                    SetYieldNextError::TimestampTooSoonToNext
                ))
            );
        });
    }

    #[test]
    fn test_set_yield_next() {
        new_test_ext().execute_with(|| {
            assert_eq!(CashYieldNext::get(), None);
            <pallet_timestamp::Module<Test>>::set_timestamp(500);
            assert_eq!(set_yield_next::<Test>(APR(100), 86400500), Ok(()));
            assert_eq!(CashYieldNext::get(), Some((APR(100), 86400500)));

            let notice_state_post: Vec<(ChainId, NoticeId, NoticeState)> =
                NoticeStates::iter().collect();
            let notice_state = notice_state_post
                .into_iter()
                .next()
                .expect("missing notice state");
            let notice = Notices::get(notice_state.0, notice_state.1);

            let expected_notice_id = NoticeId(0, 1);
            let expected_notice = Notice::FutureYieldNotice(FutureYieldNotice::Eth {
                id: expected_notice_id,
                parent: [0u8; 32],
                next_cash_yield: 100,
                next_cash_index: 1000000000000000000,
                next_cash_yield_start: 86400500,
            });

            assert_eq!(
                (
                    ChainId::Eth,
                    expected_notice_id,
                    NoticeState::Pending {
                        signature_pairs: ChainSignatureList::Eth(vec![])
                    }
                ),
                notice_state
            );

            assert_eq!(notice, Some(expected_notice.clone()));

            assert_eq!(
                LatestNotice::get(ChainId::Eth),
                Some((NoticeId(0, 1), expected_notice.hash()))
            );
        });
    }

    // TODO: Check when Timestamp was previously unset
}
