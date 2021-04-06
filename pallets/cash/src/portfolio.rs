use codec::{Decode, Encode};
use frame_support::storage::{IterableStorageDoubleMap, StorageDoubleMap};
use our_std::RuntimeDebug;

use crate::{
    chains::ChainAccount,
    core::{get_asset, get_cash_balance_with_asset_interest, get_price},
    reason::Reason,
    symbol::CASH,
    types::{AssetInfo, Balance},
    AssetBalances, AssetsWithNonZeroBalance, Config,
};

use types_derive::Types;

/// Type for representing a set of positions for an account.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub struct Portfolio {
    pub cash: Balance,
    pub positions: Vec<(AssetInfo, Balance)>,
}

impl Portfolio {
    /// Read a portfolio from storage.
    pub fn from_storage<T: Config>(account: ChainAccount) -> Result<Portfolio, Reason> {
        let cash = get_cash_balance_with_asset_interest::<T>(account)?;
        let mut positions = Vec::new();
        for (asset, _) in AssetsWithNonZeroBalance::iter_prefix(&account) {
            let asset_info = get_asset::<T>(asset)?;
            let asset_balance = AssetBalances::get(&asset, account);
            positions.push((asset_info, asset_info.as_balance(asset_balance)));
        }
        Ok(Portfolio { cash, positions })
    }

    /// Modify an asset balance to see what it does to account liquidity.
    pub fn asset_change(&self, asset_info: AssetInfo, delta: Balance) -> Result<Portfolio, Reason> {
        let mut positions = Vec::new();
        let mut seen: bool = false;
        for (info, balance) in &self.positions {
            if info.asset == asset_info.asset {
                seen = true;
                positions.push((*info, (*balance).add(delta)?))
            } else {
                positions.push((*info, (*balance)))
            };
        }
        if !seen {
            positions.push((asset_info, delta));
        }
        Ok(Portfolio {
            cash: self.cash,
            positions,
        })
    }

    /// Modify the cash principal to see what it does to account liquidity.
    pub fn cash_change(&self, delta: Balance) -> Result<Portfolio, Reason> {
        let cash = self.cash.add(delta)?;
        Ok(Portfolio {
            cash,
            positions: self.positions.clone(), // XXX
        })
    }

    /// Get the hypothetical liquidity value.
    pub fn get_liquidity<T: Config>(&self) -> Result<Balance, Reason> {
        let mut liquidity = self.cash.mul_price(get_price::<T>(CASH)?)?;
        for (info, balance) in &self.positions {
            let price = get_price::<T>(balance.units)?;
            let worth = (*balance).mul_price(price)?;
            if worth.value >= 0 {
                liquidity = liquidity.add(worth.mul_factor(info.liquidity_factor)?)?
            } else {
                liquidity = liquidity.add(worth.div_factor(info.liquidity_factor)?)?
            }
        }
        Ok(liquidity)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::{chains::*, core::*, rates::*, reason::*, symbol::*, tests::*, types::*, *};
    use pallet_oracle;
    use pallet_oracle::types::Price;

    struct TestAsset {
        asset: u8,
        ticker: &'static str,
        decimals: u8,
        balance: &'static str,
        price: &'static str,
        liquidity_factor: &'static str,
    }

    struct GetLiquidityTestCase {
        cash_index: &'static str,
        cash_principal: Option<&'static str>,
        expected_liquidity: Result<&'static str, Reason>,
        test_assets: Vec<TestAsset>,
        error_message: &'static str,
    }

    fn test_get_liquidity_test_case(case: GetLiquidityTestCase) {
        // todo: xxx finish this test
        new_test_ext().execute_with(|| {
            let account = ChainAccount::Eth([0; 20]);

            for asset_case in case.test_assets {
                let asset = ChainAsset::Eth([asset_case.asset; 20]);
                let decimals = asset_case.decimals;
                let liquidity_factor = LiquidityFactor::from_nominal(asset_case.liquidity_factor);
                let miner_shares: MinerShares = Default::default();
                let rate_model: InterestRateModel = Default::default();
                let supply_cap = AssetAmount::MAX;
                let symbol = Symbol::new(&asset_case.ticker);
                let ticker = Ticker::new(&asset_case.ticker);

                let asset_info = AssetInfo {
                    asset,
                    decimals,
                    liquidity_factor,
                    miner_shares,
                    rate_model,
                    supply_cap,
                    symbol,
                    ticker,
                };
                SupportedAssets::insert(asset, asset_info);

                let price = Price::from_nominal(ticker, asset_case.price);
                pallet_oracle::Prices::insert(ticker, price.value);

                let units = Units::from_ticker_str(&asset_case.ticker, asset_case.decimals);

                let balance = Balance::from_nominal(asset_case.balance, units);
                AssetBalances::insert(&asset, &account, balance.value);
                AssetsWithNonZeroBalance::insert(&account, &asset, ());
            }

            GlobalCashIndex::put(CashIndex::from_nominal(case.cash_index));
            if let Some(cash_principal) = case.cash_principal {
                CashPrincipals::insert(&account, CashPrincipal::from_nominal(cash_principal));
            }

            let actual = Portfolio::from_storage::<Test>(account)
                .unwrap()
                .get_liquidity::<Test>();
            let expected = match case.expected_liquidity {
                Ok(liquidity_str) => Ok(Balance::from_nominal(liquidity_str, USD)),
                Err(e) => Err(e),
            };

            assert_eq!(expected, actual, "{}", case.error_message);
        });
    }

    fn get_test_liquidity_cases() -> Vec<GetLiquidityTestCase> {
        // todo: xxx finish these test cases
        vec![
            GetLiquidityTestCase {
                cash_index: "1",
                cash_principal: None,
                expected_liquidity: Ok("0"),
                test_assets: vec![],
                error_message: "Empty account has zero liquidity",
            },
            GetLiquidityTestCase {
                cash_index: "1.1234",
                cash_principal: Some("123.123456"),
                expected_liquidity: Ok("138.316890"),
                test_assets: vec![],
                error_message: "Cash only applies principal correctly",
            },
            GetLiquidityTestCase {
                cash_index: "1.1234",
                cash_principal: Some("-123.123456"),
                expected_liquidity: Ok("-138.316890"),
                test_assets: vec![],
                error_message: "Negative cash is negative liquidity",
            },
            GetLiquidityTestCase {
                cash_index: "1.1234",
                cash_principal: None,
                expected_liquidity: Ok("82556.557312"), // last digit calcs as a 3 in gsheet
                test_assets: vec![TestAsset {
                    asset: 1,
                    ticker: "abc",
                    decimals: 6,
                    balance: "123.123456",
                    price: "987.654321",
                    liquidity_factor: "0.6789",
                }],
                error_message: "Singe asset supplied liquidity",
            },
            GetLiquidityTestCase {
                cash_index: "1.1234",
                cash_principal: None,
                expected_liquidity: Ok("-21.261329"), // sheet says -21.261334 diff -0.0000052
                test_assets: vec![
                    TestAsset {
                        asset: 1,
                        ticker: "abc",
                        decimals: 6,
                        balance: "123.123456",
                        price: "987.654321",
                        liquidity_factor: "0.6789",
                    },
                    TestAsset {
                        asset: 2,
                        ticker: "def",
                        decimals: 6,
                        balance: "-12.123456",
                        price: "987.654321",
                        liquidity_factor: "0.1450",
                    },
                ],
                error_message: "Slightly undercollateralized account",
            },
            GetLiquidityTestCase {
                cash_index: "1.1234",
                cash_principal: Some("123.123456"),
                expected_liquidity: Ok("117.055561"),
                test_assets: vec![
                    TestAsset {
                        asset: 1,
                        ticker: "abc",
                        decimals: 6,
                        balance: "123.123456",
                        price: "987.654321",
                        liquidity_factor: "0.6789",
                    },
                    TestAsset {
                        asset: 2,
                        ticker: "def",
                        decimals: 6,
                        balance: "-12.123456",
                        price: "987.654321",
                        liquidity_factor: "0.1450",
                    },
                ],
                error_message:
                    "Slightly undercollateralized by assets but with some offsetting positive cash",
            },
        ]
    }

    #[test]
    fn test_get_liquidity_all_cases() {
        get_test_liquidity_cases()
            .drain(..)
            .for_each(test_get_liquidity_test_case);
    }
}
