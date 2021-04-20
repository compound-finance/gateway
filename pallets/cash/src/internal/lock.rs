// Note: The substrate build requires these be re-exported.
use frame_support::storage::{StorageMap, StorageValue};

// Import these traits so we can interact with the substrate storage modules.
use crate::{
    chains::ChainAccount,
    internal::balance_helpers::{repay_and_supply_principal, sub_principal_amounts},
    reason::Reason,
    types::{CashIndex, CashPrincipalAmount},
    CashPrincipals, ChainCashPrincipals, Config, Event, GlobalCashIndex, Module,
    TotalCashPrincipal,
};

pub fn lock_cash_principal_internal<T: Config>(
    sender: ChainAccount,
    holder: ChainAccount,
    principal: CashPrincipalAmount,
) -> Result<(), Reason> {
    let holder_cash_principal = CashPrincipals::get(holder);
    let (holder_repay_principal, _holder_supply_principal) =
        repay_and_supply_principal(holder_cash_principal, principal)?;

    let chain_id = holder.chain_id();
    let chain_cash_principal_new = sub_principal_amounts(
        ChainCashPrincipals::get(chain_id),
        principal,
        Reason::InsufficientChainCash,
    )?;
    let holder_cash_principal_new = holder_cash_principal.add_amount(principal)?;
    let total_cash_principal_new = sub_principal_amounts(
        TotalCashPrincipal::get(),
        holder_repay_principal,
        Reason::RepayTooMuch,
    )?;

    let index: CashIndex = GlobalCashIndex::get(); // Grab cash index just for event
    ChainCashPrincipals::insert(chain_id, chain_cash_principal_new);
    CashPrincipals::insert(holder, holder_cash_principal_new);
    TotalCashPrincipal::put(total_cash_principal_new);

    <Module<T>>::deposit_event(Event::LockedCash(sender, holder, principal, index));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chains::ChainId, tests::*, types::CashPrincipal};
    use frame_support::{
        assert_err, assert_ok,
        storage::{StorageMap, StorageValue},
    };
    use our_std::{convert::TryInto, str::FromStr};

    const JARED: ChainAccount = ChainAccount::Eth([
        24, 200, 241, 34, 32, 131, 153, 116, 5, 242, 228, 130, 51, 138, 70, 80, 172, 2, 225, 214,
    ]);

    const GEOFF: ChainAccount = ChainAccount::Eth([
        129, 105, 82, 44, 44, 87, 136, 62, 142, 248, 12, 73, 138, 171, 120, 32, 218, 83, 152, 6,
    ]);

    #[test]
    fn test_lock_cash_insufficient_chain_cash() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                lock_cash_principal_internal::<Test>(
                    JARED,
                    GEOFF,
                    CashPrincipalAmount::from_nominal("1.0")
                ),
                Err(Reason::InsufficientChainCash)
            );
        });
    }

    #[test]
    fn test_lock_cash_repay_too_much() {
        new_test_ext().execute_with(|| {
            let once_principal_amount = CashPrincipalAmount::from_nominal("1.0");
            let twice_principal_amount = CashPrincipalAmount::from_nominal("2.0");
            let twice_principal: CashPrincipal = twice_principal_amount.try_into().unwrap();

            // As far as I can tell, this is an impossible case in practice
            // Basically we're saying that there's 2 CASH on Eth, but 1 CASH in total.
            CashPrincipals::insert(JARED, twice_principal.negate());
            ChainCashPrincipals::insert(ChainId::Eth, twice_principal_amount);
            TotalCashPrincipal::put(once_principal_amount);

            assert_eq!(
                lock_cash_principal_internal::<Test>(GEOFF, JARED, twice_principal_amount),
                Err(Reason::RepayTooMuch)
            );
        });
    }

    #[test]
    fn test_lock_cash() {
        new_test_ext().execute_with(|| {
            let once_principal_amount = CashPrincipalAmount::from_nominal("1.0");
            let twice_principal_amount = CashPrincipalAmount::from_nominal("2.0");
            let thrice_principal_amount = CashPrincipalAmount::from_nominal("3.0");
            let twice_principal: CashPrincipal = twice_principal_amount.try_into().unwrap();
            let thrice_principal: CashPrincipal = thrice_principal_amount.try_into().unwrap();

            CashPrincipals::insert(JARED, thrice_principal.negate());
            ChainCashPrincipals::insert(ChainId::Eth, thrice_principal_amount);
            TotalCashPrincipal::put(thrice_principal_amount);

            assert_eq!(
                lock_cash_principal_internal::<Test>(GEOFF, JARED, once_principal_amount),
                Ok(())
            );

            assert_eq!(
                ChainCashPrincipals::get(ChainId::Eth),
                twice_principal_amount
            );
            assert_eq!(CashPrincipals::get(JARED), twice_principal.negate());
            assert_eq!(TotalCashPrincipal::get(), twice_principal_amount);
        });
    }

    #[test]
    fn test_lock_cash_event() {
        new_test_ext().execute_with(|| {
            let once_principal_amount = CashPrincipalAmount::from_nominal("1.0");
            let once_principal: CashPrincipal = once_principal_amount.try_into().unwrap();
            let cash_index = CashIndex::from_nominal("1.1");

            CashPrincipals::insert(JARED, once_principal.negate());
            ChainCashPrincipals::insert(ChainId::Eth, once_principal_amount);
            TotalCashPrincipal::put(once_principal_amount);
            GlobalCashIndex::put(cash_index);

            let events_pre: Vec<_> = System::events().into_iter().collect();

            assert_eq!(
                lock_cash_principal_internal::<Test>(GEOFF, JARED, once_principal_amount),
                Ok(())
            );

            let events_post: Vec<_> = System::events().into_iter().collect();
            assert_eq!(events_pre.len() + 1, events_post.len());

            let locked_cash_event = events_post.into_iter().next().unwrap();

            assert_eq!(
                mock::Event::pallet_cash(crate::Event::LockedCash(
                    GEOFF,
                    JARED,
                    once_principal_amount,
                    cash_index
                )),
                locked_cash_event.event
            );
        })
    }

    #[test]
    fn lock_cash_without_chain_cash_or_total_cash_fails() -> Result<(), Reason> {
        let jared = ChainAccount::from_str("Eth:0x18c8F1222083997405F2E482338A4650ac02e1d6")?;
        let geoff = ChainAccount::from_str("Eth:0x8169522c2c57883e8ef80c498aab7820da539806")?;
        let lock_principal = CashPrincipalAmount::from_nominal("100");
        new_test_ext().execute_with(|| {
            assert_err!(
                lock_cash_principal_internal::<Test>(jared, jared, lock_principal),
                Reason::InsufficientChainCash
            );
            ChainCashPrincipals::insert(ChainId::Eth, lock_principal);
            assert_ok!(lock_cash_principal_internal::<Test>(
                jared,
                jared,
                lock_principal
            ));
            assert_eq!(
                ChainCashPrincipals::get(ChainId::Eth),
                CashPrincipalAmount(0)
            );

            ChainCashPrincipals::insert(ChainId::Eth, lock_principal);
            CashPrincipals::insert(&geoff, CashPrincipal::from_nominal("-1"));
            assert_err!(
                lock_cash_principal_internal::<Test>(geoff, geoff, lock_principal),
                Reason::RepayTooMuch
            );
            TotalCashPrincipal::put(CashPrincipalAmount::from_nominal("1"));
            assert_ok!(lock_cash_principal_internal::<Test>(
                geoff,
                geoff,
                lock_principal
            ));
            assert_eq!(TotalCashPrincipal::get(), CashPrincipalAmount(0));

            Ok(())
        })
    }
}
