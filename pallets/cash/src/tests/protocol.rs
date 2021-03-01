use super::*;

#[test]
fn upload_transfer_download() -> Result<(), Reason> {
    let jared = ChainAccount::from_str("Eth:0x18c8F1222083997405F2E482338A4650ac02e1d6")?;
    let geoff = ChainAccount::from_str("Eth:0x8169522c2c57883e8ef80c498aab7820da539806")?;
    let lock_amount = qty!("1000", UNI);
    new_test_ext().execute_with(|| {
        Prices::insert(UNI.ticker, Price::from_nominal(UNI.ticker, "0.99").value);
        SupportedAssets::insert(&Uni, uni);

        // Upload

        assert_ok!(core::lock_internal::<Test>(uni, jared, jared, lock_amount));
        assert_eq!(CashPrincipals::get(&jared), CashPrincipal(0));
        assert_eq!(CashPrincipals::get(&geoff), CashPrincipal(0));
        assert_eq!(AssetBalances::get(&Uni, &jared), bal!("1000", UNI).value);

        // Transfer

        assert_err!(
            core::transfer_internal::<Test>(uni, jared, geoff, lock_amount),
            Reason::InsufficientLiquidity // transfer fee
        );
        assert_ok!(core::transfer_internal::<Test>(
            uni,
            jared,
            geoff,
            qty!("998", UNI)
        ));
        assert_eq!(
            CashPrincipals::get(&jared),
            CashPrincipal::from_nominal("-0.01")
        );
        assert_eq!(CashPrincipals::get(&geoff), CashPrincipal(0));
        assert_eq!(AssetBalances::get(&Uni, &jared), bal!("2", UNI).value);
        assert_eq!(AssetBalances::get(&Uni, &geoff), bal!("998", UNI).value);

        // Download

        assert_ok!(core::extract_internal::<Test>(
            uni,
            geoff,
            jared,
            qty!("998", UNI)
        ));
        assert_eq!(
            CashPrincipals::get(&jared),
            CashPrincipal::from_nominal("-0.01")
        );
        assert_eq!(CashPrincipals::get(&geoff), CashPrincipal(0));
        assert_eq!(AssetBalances::get(&Uni, &jared), bal!("2", UNI).value);
        assert_eq!(AssetBalances::get(&Uni, &geoff), 0);

        assert_err!(
            core::extract_internal::<Test>(uni, jared, geoff, qty!("2", UNI)),
            Reason::InsufficientLiquidity
        );

        assert_ok!(core::extract_internal::<Test>(
            uni,
            jared,
            jared,
            qty!("1.9", UNI)
        ));
        assert_eq!(
            CashPrincipals::get(&jared),
            CashPrincipal::from_nominal("-0.01")
        );
        assert_eq!(CashPrincipals::get(&geoff), CashPrincipal(0));
        assert_eq!(AssetBalances::get(&Uni, &jared), bal!("0.1", UNI).value);
        assert_eq!(AssetBalances::get(&Uni, &geoff), 0);

        assert_err!(
            core::extract_internal::<Test>(uni, jared, jared, qty!("1", UNI)),
            Reason::MinTxValueNotMet
        );

        Ok(())
    })
}

#[test]
fn lock_cash_without_chain_cash_or_total_cash_fails() -> Result<(), Reason> {
    let jared = ChainAccount::from_str("Eth:0x18c8F1222083997405F2E482338A4650ac02e1d6")?;
    let geoff = ChainAccount::from_str("Eth:0x8169522c2c57883e8ef80c498aab7820da539806")?;
    let lock_principal = CashPrincipalAmount::from_nominal("100");
    new_test_ext().execute_with(|| {
        assert_err!(
            core::lock_cash_principal_internal::<Test>(jared, jared, lock_principal),
            Reason::InsufficientChainCash
        );
        ChainCashPrincipals::insert(ChainId::Eth, lock_principal);
        assert_ok!(core::lock_cash_principal_internal::<Test>(
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
            core::lock_cash_principal_internal::<Test>(geoff, geoff, lock_principal),
            Reason::RepayTooMuch
        );
        TotalCashPrincipal::put(CashPrincipalAmount::from_nominal("1"));
        assert_ok!(core::lock_cash_principal_internal::<Test>(
            geoff,
            geoff,
            lock_principal
        ));
        assert_eq!(TotalCashPrincipal::get(), CashPrincipalAmount(0));

        Ok(())
    })
}
