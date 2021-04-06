use crate::{
    core::get_now,
    internal,
    params::MIN_NEXT_SYNC_TIME,
    rates::APR,
    reason::Reason,
    require,
    types::{CashIndex, Timestamp},
    CashYield, CashYieldNext, Config, Event, GlobalCashIndex, Module,
};
use codec::{Decode, Encode};
use frame_support::storage::StorageValue;
use our_std::Debuggable;

use types_derive::Types;

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Debuggable, Types)]
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

pub fn set_yield_next<T: Config>(
    next_yield: APR,
    next_yield_start: Timestamp,
) -> Result<(), Reason> {
    let now = get_now::<T>();
    let change_in_time = next_yield_start
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
        next_yield_start >= now + MIN_NEXT_SYNC_TIME,
        Reason::SetYieldNextError(SetYieldNextError::TimestampTooSoonToNow)
    );

    // XXX fix this up
    // Read NextYieldIndex=GetCashYieldIndexAt(NextAPRStart)
    let next_yield_index: CashIndex = get_cash_yield_index_after::<T>(change_in_time)?;

    CashYieldNext::put((next_yield, next_yield_start));

    <Module<T>>::deposit_event(Event::SetYieldNext(next_yield, next_yield_start));

    internal::notices::dispatch_future_yield_notice::<T>(
        next_yield,
        next_yield_index,
        next_yield_start,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chains::*, notices::*, tests::*, LatestNotice, NoticeStates, Notices};
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

            // bumps era
            let expected_notice_id = NoticeId(1, 0);
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
                Some((expected_notice_id, expected_notice.hash()))
            );
        });
    }

    // TODO: Check when Timestamp was previously unset
}
