use crate::{
    core::get_recent_timestamp,
    internal,
    params::MIN_NEXT_SYNC_TIME,
    rates::APR,
    reason::{MathError, Reason},
    require,
    types::{CashIndex, Timestamp},
    CashYield, CashYieldNext, Config, Event, GlobalCashIndex, Pallet,
};
use codec::{Decode, Encode};
use frame_support::storage::StorageValue;
use our_std::Debuggable;
use types_derive::Types;

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Debuggable, Types)]
pub enum SetYieldNextError {
    NotEnoughTimeToSyncBeforeNext,
    NotEnoughTimeToSyncBeforeCancel,
}

fn get_cash_yield_index_after<T: Config>(dt: Timestamp) -> Result<CashIndex, Reason> {
    let cash_yield = CashYield::get();
    let cash_index_old = GlobalCashIndex::get();
    let increment = cash_yield.compound(dt)?;
    Ok(cash_index_old.increment(increment.into())?)
}

pub fn set_yield_next<T: Config>(
    next_yield: APR,
    next_yield_start: Timestamp,
) -> Result<(), Reason> {
    let now = get_recent_timestamp::<T>()?;
    let min_t = now
        .checked_add(MIN_NEXT_SYNC_TIME)
        .ok_or(MathError::Overflow)?;

    require!(
        next_yield_start >= min_t,
        SetYieldNextError::NotEnoughTimeToSyncBeforeNext.into()
    );

    if let Some((_, next_start)) = CashYieldNext::get() {
        require!(
            next_start >= min_t,
            SetYieldNextError::NotEnoughTimeToSyncBeforeCancel.into()
        );
    }

    let dt = next_yield_start
        .checked_sub(now)
        .ok_or(MathError::Underflow)?;
    let next_yield_index = get_cash_yield_index_after::<T>(dt)?;

    CashYieldNext::put((next_yield, next_yield_start));

    <Pallet<T>>::deposit_event(Event::SetYieldNext(next_yield, next_yield_start));

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
    fn test_too_soon_to_next() {
        new_test_ext().execute_with(|| {
            <pallet_timestamp::Pallet<Test>>::set_timestamp(500);
            assert_eq!(
                set_yield_next::<Test>(APR(100), 501),
                Err(SetYieldNextError::NotEnoughTimeToSyncBeforeNext.into())
            );
        });
    }

    #[test]
    fn test_too_soon_to_cancel() {
        new_test_ext().execute_with(|| {
            <pallet_timestamp::Pallet<Test>>::set_timestamp(1);
            CashYieldNext::put((APR(100), 500));
            assert_eq!(
                set_yield_next::<Test>(APR(100), MIN_NEXT_SYNC_TIME + 1),
                Err(SetYieldNextError::NotEnoughTimeToSyncBeforeCancel.into())
            );
        });
    }

    #[test]
    fn test_set_yield_next() {
        new_test_ext().execute_with(|| {
            assert_eq!(CashYieldNext::get(), None);
            <pallet_timestamp::Pallet<Test>>::set_timestamp(500);
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

            // Check emitted `SetYieldNext` event
            let mut events_iter = System::events().into_iter();
            let yield_next_event = events_iter.next().unwrap();
            assert_eq!(
                mock::Event::pallet_cash(crate::Event::SetYieldNext(APR(100), 86400500)),
                yield_next_event.event
            );
            // Check emitted `Notice` event
            let notice_event = events_iter.next().unwrap();
            let expected_notice_encoded = expected_notice.encode_notice();
            assert_eq!(
                mock::Event::pallet_cash(crate::Event::Notice(
                    expected_notice_id,
                    expected_notice.clone(),
                    expected_notice_encoded
                )),
                notice_event.event
            );
        });
    }

    // TODO: Check when Timestamp was previously unset
}
