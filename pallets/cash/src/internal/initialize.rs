use crate::{
    chains::ChainAsset,
    factor::Factor,
    internal, log,
    rates::APR,
    reason::Reason,
    types::{AssetIndex, CashPrincipalAmount, Quantity, Timestamp, CASH},
    BorrowIndices, CashPrincipals, CashYield, CashYieldNext, Config, GlobalCashIndex,
    LastBlockTimestamp, LastMinerSharePrincipal, LastYieldCashIndex, LastYieldTimestamp,
    SupplyIndices, SupportedAssets, TotalBorrowAssets, TotalCashPrincipal, TotalSupplyAssets,
};
use frame_support::storage::{IterableStorageMap, StorageMap, StorageValue};

/// Block initialization step that can fail.
pub fn on_initialize_internal<T: Config>(
    now: Timestamp,
    last_yield_timestamp: Timestamp,
    last_block_timestamp: Timestamp,
) -> Result<(), Reason> {
    // XXX re-evaluate how we do time, we don't really want this to be zero but there may
    // not actually be any good way to do "current" time per-se so what we have here is more like
    // the last block's time and the block before
    // XXX also we should try to inject Now (and previous) for tests instead of taking different path
    if now == 0 {
        return Err(Reason::TimeTravelNotAllowed);
    }
    if last_yield_timestamp == 0 || last_block_timestamp == 0 {
        // this is the first time we have seen a valid time, set it for LastYield and LastBlock
        if last_yield_timestamp == 0 {
            LastYieldTimestamp::put(now);
        }
        if last_block_timestamp == 0 {
            LastBlockTimestamp::put(now);
        }

        // All dt will be 0 so bail out here, no interest accrued in this block.
        return Ok(());
    }

    // Iterate through listed assets, summing the CASH principal they generated/paid last block
    let dt_since_last_block = now
        .checked_sub(last_block_timestamp)
        .ok_or(Reason::TimeTravelNotAllowed)?;
    let dt_since_last_yield = now
        .checked_sub(last_yield_timestamp)
        .ok_or(Reason::TimeTravelNotAllowed)?;
    let mut cash_principal_supply_increase = CashPrincipalAmount::ZERO;
    let mut cash_principal_borrow_increase = CashPrincipalAmount::ZERO;

    let last_block_cash_index = GlobalCashIndex::get();
    let last_yield_cash_index = LastYieldCashIndex::get();
    let cash_yield = CashYield::get();
    let price_cash = internal::assets::get_price_or_zero::<T>(CASH);

    let mut asset_updates: Vec<(ChainAsset, AssetIndex, AssetIndex)> = Vec::new();
    for (asset, asset_info) in SupportedAssets::iter() {
        let (asset_cost, asset_yield) = internal::assets::get_rates::<T>(asset)?;
        let asset_units = asset_info.units();
        let price_asset = internal::assets::get_price_or_zero::<T>(asset_units);
        let price_ratio = Factor::ratio(price_asset, price_cash)?;
        let cash_borrow_principal_per_asset = last_block_cash_index
            .cash_principal_per_asset(asset_cost.simple(dt_since_last_block)?, price_ratio)?;
        let cash_hold_principal_per_asset = last_block_cash_index
            .cash_principal_per_asset(asset_yield.simple(dt_since_last_block)?, price_ratio)?;

        let supply_index = SupplyIndices::get(&asset);
        let borrow_index = BorrowIndices::get(&asset);
        let supply_index_new = supply_index.increment(cash_hold_principal_per_asset)?;
        let borrow_index_new = borrow_index.increment(cash_borrow_principal_per_asset)?;

        let supply_asset = Quantity::new(TotalSupplyAssets::get(asset), asset_units);
        let borrow_asset = Quantity::new(TotalBorrowAssets::get(asset), asset_units);
        cash_principal_supply_increase = cash_principal_supply_increase
            .add(cash_hold_principal_per_asset.cash_principal_amount(supply_asset)?)?;
        cash_principal_borrow_increase = cash_principal_borrow_increase
            .add(cash_borrow_principal_per_asset.cash_principal_amount(borrow_asset)?)?;

        asset_updates.push((asset.clone(), supply_index_new, borrow_index_new));
    }

    // Pay miners and update the CASH interest index on CASH itself
    if cash_yield == APR::ZERO {
        log!("Cash yield is zero. No interest earned on cash in this block.");
    }

    let total_cash_principal = TotalCashPrincipal::get();

    let increment = cash_yield.compound(dt_since_last_yield)?;
    if increment == Factor::ONE {
        log!("Index increment = 1. No interest on cash earned in this block!")
    }
    let cash_index_new = last_yield_cash_index.increment(increment.into())?;
    let total_cash_principal_new = total_cash_principal.add(cash_principal_borrow_increase)?;
    let miner_share_principal =
        cash_principal_borrow_increase.sub(cash_principal_supply_increase)?;

    let last_miner = internal::miner::get_some_miner::<T>(); // Miner not yet set for this block, so this is "last miner"
    let last_miner_share_principal = LastMinerSharePrincipal::get();
    let miner_cash_principal_old = CashPrincipals::get(&last_miner);
    let miner_cash_principal_new =
        miner_cash_principal_old.add_amount(last_miner_share_principal)?;

    // * BEGIN STORAGE ALL CHECKS AND FAILURES MUST HAPPEN ABOVE * //

    CashPrincipals::insert(last_miner, miner_cash_principal_new);
    log!(
        "Miner={:?} received {:?} principal for mining last block",
        String::from(last_miner),
        last_miner_share_principal
    );

    for (asset, new_supply_index, new_borrow_index) in asset_updates.drain(..) {
        SupplyIndices::insert(asset.clone(), new_supply_index);
        BorrowIndices::insert(asset, new_borrow_index);
    }

    GlobalCashIndex::put(cash_index_new);
    TotalCashPrincipal::put(total_cash_principal_new);
    LastMinerSharePrincipal::put(miner_share_principal);
    LastBlockTimestamp::put(now);

    // Possibly rotate in any scheduled next CASH rate
    if let Some((next_apr, next_start)) = CashYieldNext::get() {
        if next_start <= now {
            LastYieldTimestamp::put(next_start);
            LastYieldCashIndex::put(cash_index_new);
            CashYield::put(next_apr);
            CashYieldNext::kill();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn test_on_initialize() {
        new_test_ext().execute_with(|| {
            // XXX how to inject miner?
            let miner = ChainAccount::Eth([0; 20]);
            let asset = Eth;
            let asset_info = AssetInfo {
                rate_model: InterestRateModel::new_kink(0, 2500, Factor::from_nominal("0.5"), 5000),
                miner_shares: MinerShares::from_nominal("0.02"),
                ..AssetInfo::minimal(asset, ETH)
            };
            // XXX how to inject now / last yield timestamp?
            let last_yield_timestamp = 10;
            let now = last_yield_timestamp + MILLISECONDS_PER_YEAR / 4; // 3 months go by

            SupportedAssets::insert(&asset, asset_info);
            GlobalCashIndex::put(CashIndex::from_nominal("1.123"));
            LastYieldCashIndex::put(CashIndex::from_nominal("1.123"));
            SupplyIndices::insert(&asset, AssetIndex::from_nominal("1234"));
            BorrowIndices::insert(&asset, AssetIndex::from_nominal("1345"));
            TotalSupplyAssets::insert(asset.clone(), asset_info.as_quantity_nominal("300").value);
            TotalBorrowAssets::insert(asset.clone(), asset_info.as_quantity_nominal("150").value);
            CashYield::put(APR::from_nominal("0.24")); // 24% APR big number for easy to see interest
            TotalCashPrincipal::put(CashPrincipalAmount::from_nominal("450000")); // 450k cash principal
            CashPrincipals::insert(&miner, CashPrincipal::from_nominal("1"));
            pallet_oracle::Prices::insert(
                asset_info.ticker,
                1450_000000 as pallet_oracle::types::AssetPrice,
            ); // $1450 eth

            let result =
                on_initialize_internal::<Test>(now, last_yield_timestamp, last_yield_timestamp);
            assert_eq!(result, Ok(()));

            assert_eq!(
                SupplyIndices::get(&asset),
                AssetIndex::from_nominal("1273.542520035618878005")
            );
            assert_eq!(
                BorrowIndices::get(&asset),
                AssetIndex::from_nominal("1425.699020480854853072")
            );
            // note - the cash index number below is quite round due to the polynomial nature of
            // our approximation and the fact that the ratio in this case worked out to be a
            // base 10 number that terminates in that many digits.
            assert_eq!(
                GlobalCashIndex::get(),
                CashIndex::from_nominal("1.192441828000000000")
            );
            assert_eq!(
                TotalCashPrincipal::get(),
                CashPrincipalAmount::from_nominal("462104.853072")
            );
            assert_eq!(
                CashPrincipals::get(&miner),
                CashPrincipal::from_nominal("1.000000")
            );
            // Run again to set miner true principal
            assert_eq!(
                on_initialize_internal::<Test>(now, last_yield_timestamp, last_yield_timestamp),
                Ok(())
            );
            assert_eq!(
                CashPrincipals::get(&miner),
                CashPrincipal::from_nominal("243.097062")
            );
        });
    }

    #[test]
    fn test_on_initialize_next_yield_progression() {
        new_test_ext().execute_with(|| {
            let now = MILLISECONDS_PER_YEAR; // we are at year 1 (from epoc)
            let last_yield_timestamp = 3 * MILLISECONDS_PER_YEAR / 4; // last time we set the yield was 3 months ago
            let last_block_timestamp = now - 6 * 1000; // last block was 6 seconds ago
            let next_yield_timestamp = now + 6 * 1000; // we are going to set the next yield in the next block 6 seconds from "now"
            let global_cash_index_initial = CashIndex::from_nominal("1.123");
            let last_yield_cash_index_initial = CashIndex::from_nominal("1.111");
            let cash_yield_initial = APR::from_nominal("0.24");
            let cash_yield_next = APR::from_nominal("0.32");

            CashYield::put(cash_yield_initial);
            CashYieldNext::put((cash_yield_next, next_yield_timestamp));
            GlobalCashIndex::put(global_cash_index_initial);
            LastYieldCashIndex::put(last_yield_cash_index_initial);
            LastYieldTimestamp::put(last_yield_timestamp);

            let result =
                on_initialize_internal::<Test>(now, last_yield_timestamp, last_block_timestamp);
            assert_eq!(result, Ok(()));

            let increment_expected = cash_yield_initial
                .compound(now - last_yield_timestamp)
                .expect("could not compound interest during expected calc");
            let new_index_expected = last_yield_cash_index_initial
                .increment(increment_expected.into())
                .expect("could not increment index value during expected calc");
            let new_index_actual = GlobalCashIndex::get();
            assert_eq!(
                new_index_expected, new_index_actual,
                "current yield was not used to calculate the cash index increment"
            );
            let (next_yield_actual, next_yield_timestamp_actual) =
                CashYieldNext::get().expect("cash yield next was cleared unexpectedly");
            assert_eq!(
                next_yield_actual, cash_yield_next,
                "cash yield next yield was modified unexpectedly"
            );
            assert_eq!(
                next_yield_timestamp_actual, next_yield_timestamp,
                "cash yield next timestamp was modified unexpectedly"
            );
            assert_eq!(
                LastYieldTimestamp::get(),
                last_yield_timestamp,
                "last yield timestamp changed unexpectedly"
            );
            assert_eq!(
                LastYieldCashIndex::get(),
                last_yield_cash_index_initial,
                "last yield cash index was updated unexpectedly"
            );

            // simulate "a block goes by" in time
            let last_block_timestamp = now;
            let now = next_yield_timestamp;

            let result =
                on_initialize_internal::<Test>(now, last_yield_timestamp, last_block_timestamp);
            assert_eq!(result, Ok(()));

            let increment_expected = cash_yield_initial
                .compound(now - last_yield_timestamp)
                .expect("could not compound interest during expected calc");
            let new_index_expected = last_yield_cash_index_initial
                .increment(increment_expected.into())
                .expect("could not increment index value during expected calc");
            let new_index_actual = GlobalCashIndex::get();
            assert_eq!(
                new_index_expected, new_index_actual,
                "current yield was not used to calculate the cash index increment"
            );
            assert!(CashYieldNext::get().is_none(), "next yield was not cleared");
            let actual_cash_yield = CashYield::get();
            assert_eq!(
                actual_cash_yield, cash_yield_next,
                "the current yield was not updated to next yield"
            );
            assert_eq!(
                LastYieldTimestamp::get(),
                next_yield_timestamp,
                "last yield timestamp was not updated"
            );
            assert_eq!(
                LastYieldCashIndex::get(),
                new_index_actual,
                "last yield cash index was not updated correctly"
            );

            // simulate "a block goes by" in time
            let last_block_timestamp = now;
            let now = now + 6 * 1000;
            let new_cash_index_baseline = new_index_actual;

            let result =
                on_initialize_internal::<Test>(now, next_yield_timestamp, last_block_timestamp);
            assert_eq!(result, Ok(()));

            let increment_expected = cash_yield_next
                .compound(now - next_yield_timestamp)
                .expect("could not compound interest during expected calc");
            let new_index_expected = new_cash_index_baseline
                .increment(increment_expected.into())
                .expect("could not increment index value during expected calc");
            let new_index_actual = GlobalCashIndex::get();
            assert_eq!(
                new_index_expected, new_index_actual,
                "current yield was not used to calculate the cash index increment"
            );
            assert!(CashYieldNext::get().is_none(), "next yield was not cleared");
            let actual_cash_yield = CashYield::get();
            assert_eq!(
                actual_cash_yield, cash_yield_next,
                "the current yield was not updated to next yield"
            );
            assert_eq!(
                LastYieldTimestamp::get(),
                next_yield_timestamp,
                "last yield timestamp was modified unexpectedly"
            );
            assert_eq!(
                LastYieldCashIndex::get(),
                new_cash_index_baseline,
                "last yield cash index was updated unexpectedly"
            );
        });
    }
}
