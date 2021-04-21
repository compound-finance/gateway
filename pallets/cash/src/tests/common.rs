use crate::tests::{assets::*, *};

pub fn init_eth_asset() -> Result<ChainAsset, Reason> {
    pallet_oracle::Prices::insert(ETH.ticker, Price::from_nominal(ETH.ticker, "2000.00").value);
    SupportedAssets::insert(&Eth, eth);

    Ok(Eth)
}

#[allow(dead_code)]
pub fn init_uni_asset() -> Result<ChainAsset, Reason> {
    pallet_oracle::Prices::insert(
        UNI.ticker,
        Price::from_nominal(UNI.ticker, "60000.00").value,
    );
    SupportedAssets::insert(&Uni, uni);

    Ok(Uni)
}

pub fn init_wbtc_asset() -> Result<ChainAsset, Reason> {
    pallet_oracle::Prices::insert(
        WBTC.ticker,
        Price::from_nominal(WBTC.ticker, "60000.00").value,
    );
    SupportedAssets::insert(&Wbtc, wbtc);

    Ok(Wbtc)
}

pub fn init_asset_balance(asset: ChainAsset, account: ChainAccount, balance: AssetBalance) {
    AssetBalances::insert(asset, account, balance);
    if balance >= 0 {
        TotalSupplyAssets::insert(
            asset,
            (TotalSupplyAssets::get(asset) as i128 + balance) as u128,
        );
    } else {
        TotalBorrowAssets::insert(
            asset,
            (TotalBorrowAssets::get(asset) as i128 + balance) as u128,
        );
    }
    AssetsWithNonZeroBalance::insert(account, asset, ());
}

pub fn init_cash(account: ChainAccount, amount: CashPrincipal) {
    if amount.0 < 0 {
        init_cash(ChainAccount::Eth([0; 20]), amount.negate());
    }
    let pre: CashPrincipal = TotalCashPrincipal::get().try_into().unwrap();

    TotalCashPrincipal::set(
        pre.add(amount)
            .expect("cash setup overflow")
            .try_into()
            .unwrap(),
    );
    CashPrincipals::insert(account, amount);
}
