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

/// Type for representing a set of positions for an account.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Portfolio {
    pub cash: Balance,
    pub positions: Vec<(AssetInfo, Balance)>,
}

impl Portfolio {
    /// Read a portfolio from storage.
    pub fn from_storage<T: Config>(account: ChainAccount) -> Result<Portfolio, Reason> {
        let cash = get_cash_balance_with_asset_interest(account)?;
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
        let mut liquidity = self.cash.mul_price(get_price::<T>(CASH))?;
        for (info, balance) in &self.positions {
            let price = get_price::<T>(balance.units);
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

// XXX fix/convert these tests
// #[cfg(test)]
// pub mod tests {
//     use super::*;
//     use crate::{chains::*, core::*, mock::*, rates::*, reason::*, symbol::*, *};

//     #[test]
//     fn test_from_storage() {
//         new_test_ext().execute_with(|| {
//             let asset1 = ChainAsset::Eth([238; 20]);
//             let asset2 = ChainAsset::Eth([100; 20]);
//             let account = ChainAccount::Eth([0; 20]);
//             let asset1_symbol = Symbol::new("ETH", 18);
//             let asset2_symbol = Symbol::new("COMP", 6);
//             let asset1_balance =
//                 Quantity::from_nominal(asset1_symbol, "11.123456789").1 as AssetBalance;
//             let cash_principal = CashPrincipal::from_nominal("100");

//             // could also set CashPrincipals
//             Prices::insert(asset1_symbol, 1451_123456); // $1451.123456
//             Prices::insert(asset2_symbol, 500_789123); // $1451.123456
//             AssetSymbols::insert(&asset1, asset1_symbol);
//             AssetSymbols::insert(&asset2, asset2_symbol);
//             AssetsWithNonZeroBalance::insert(account.clone(), vec![asset1.clone(), asset2.clone()]);

//             AssetBalances::insert(asset1.clone(), account.clone(), asset1_balance);
//             CashPrincipals::insert(&account, cash_principal);

//             let expected = Portfolio {
//                 positions: vec![
//                     SignedQuantity(asset1_symbol, asset1_balance),
//                     SignedQuantity(asset2_symbol, 0),
//                 ],
//                 cash_principal: cash_principal,
//             };

//             let actual = Portfolio::from_storage(&account).unwrap();
//             assert_eq!(actual, expected);

//             // test seen
//             let change_in_balance = SignedQuantity(asset1_symbol, asset1_balance);
//             let actual = actual.asset_balance_change(change_in_balance).unwrap();
//             let expected = Portfolio {
//                 positions: vec![
//                     SignedQuantity(asset1_symbol, asset1_balance * 2),
//                     SignedQuantity(asset2_symbol, 0),
//                 ],
//                 cash_principal: cash_principal,
//             };
//             assert_eq!(actual, expected);

//             // test not seen
//             let asset3_symbol = Symbol::new("FOO", 6);
//             let asset3_balance = SignedQuantity(asset3_symbol, 1234567);
//             let actual = actual.asset_balance_change(asset3_balance).unwrap();
//             let expected = Portfolio {
//                 positions: vec![
//                     SignedQuantity(asset1_symbol, asset1_balance * 2),
//                     SignedQuantity(asset2_symbol, 0),
//                     asset3_balance,
//                 ],
//                 cash_principal: cash_principal,
//             };
//             assert_eq!(actual, expected);

//             // test modify cash
//             let change_in_principal = CashPrincipal::from_nominal("123456");
//             let actual = actual.cash_principal_change(change_in_principal).unwrap();
//             let expected = Portfolio {
//                 positions: vec![
//                     SignedQuantity(asset1_symbol, asset1_balance * 2),
//                     SignedQuantity(asset2_symbol, 0),
//                     asset3_balance,
//                 ],
//                 cash_principal: cash_principal.add(change_in_principal).unwrap(),
//             };
//             assert_eq!(actual, expected);
//         });
//     }

//     struct TestAsset {
//         asset: u8,
//         ticker: &'static str,
//         decimals: u8,
//         balance: &'static str,
//         price: &'static str,
//         liquidity_factor: &'static str,
//     }

//     struct GetLiquidityTestCase {
//         cash_index: &'static str,
//         cash_principal: Option<&'static str>,
//         expected_liquidity: Result<&'static str, Reason>,
//         test_assets: Vec<TestAsset>,
//         error_message: &'static str,
//     }

//     fn test_get_liquidity_test_case(case: GetLiquidityTestCase) {
//         // todo: xxx finish this test
//         new_test_ext().execute_with(|| {
//             let account = ChainAccount::Eth([0; 20]);
//             let mut assets_with_nonzero_balances = Vec::new();

//             for asset_case in case.test_assets {
//                 let asset = ChainAsset::Eth([asset_case.asset; 20]);
//                 assets_with_nonzero_balances.push(asset);

//                 let symbol = Symbol::new(&asset_case.ticker, asset_case.decimals);
//                 let price = Price::from_nominal(symbol, asset_case.price);

//                 AssetSymbols::insert(&asset, symbol);
//                 Prices::insert(&symbol, price.value());
//                 let balance = SignedQuantity::from_nominal(symbol, asset_case.balance);
//                 AssetBalances::insert(&asset, &account, balance.value());
//                 LiquidityFactors::insert(
//                     &symbol,
//                     LiquidityFactor::from_nominal(asset_case.liquidity_factor),
//                 )
//             }
//             AssetsWithNonZeroBalance::insert(&account, assets_with_nonzero_balances);

//             GlobalCashIndex::put(CashIndex::from_nominal(case.cash_index));
//             if let Some(cash_principal) = case.cash_principal {
//                 CashPrincipals::insert(&account, CashPrincipal::from_nominal(cash_principal));
//             }

//             let actual = Portfolio::from_storage(&account).unwrap().get_liquidity();
//             let expected = match case.expected_liquidity {
//                 Ok(liquidity_str) => Ok(SignedQuantity::from_nominal(USD, liquidity_str)),
//                 Err(e) => Err(e),
//             };

//             assert_eq!(expected, actual, "{}", case.error_message);
//         });
//     }

//     fn get_test_liquidity_cases() -> Vec<GetLiquidityTestCase> {
//         // todo: xxx finish these test cases
//         vec![
//             /*GetLiquidityTestCase {
//                 cash_index: "1",
//                 cash_principal: None,
//                 expected_liquidity: Ok("0"),
//                 test_assets: vec![],
//                 error_message: "Empty account has zero liquidity",
//             },
//             GetLiquidityTestCase {
//                 cash_index: "1.1234",
//                 cash_principal: Some("123.123456"),
//                 expected_liquidity: Ok("138.316890"),
//                 test_assets: vec![],
//                 error_message: "Cash only applies principal correctly",
//             },
//             GetLiquidityTestCase {
//                 cash_index: "1.1234",
//                 cash_principal: Some("-123.123456"),
//                 expected_liquidity: Ok("-138.316890"),
//                 test_assets: vec![],
//                 error_message: "Negative cash is negative liquidity",
//             },
//             GetLiquidityTestCase {
//                 cash_index: "1.1234",
//                 cash_principal: None,
//                 expected_liquidity: Ok("82556.557312"), // last digit calcs as a 3 in gsheet
//                 test_assets: vec![TestAsset {
//                     asset: 1,
//                     ticker: "abc",
//                     decimals: 6,
//                     balance: "123.123456",
//                     price: "987.654321",
//                     liquidity_factor: "0.6789",
//                 }],
//                 error_message: "Singe asset supplied liquidity",
//             },*/
//             GetLiquidityTestCase {
//                 cash_index: "1.1234",
//                 cash_principal: None,
//                 expected_liquidity: Ok("-21.261329"), // sheet says -21.261334 diff -0.0000052
//                 test_assets: vec![
//                     TestAsset {
//                         asset: 1,
//                         ticker: "abc",
//                         decimals: 6,
//                         balance: "123.123456",
//                         price: "987.654321",
//                         liquidity_factor: "0.6789",
//                     },
//                     TestAsset {
//                         asset: 2,
//                         ticker: "def",
//                         decimals: 6,
//                         balance: "-12.123456",
//                         price: "987.654321",
//                         liquidity_factor: "0.1450",
//                     },
//                 ],
//                 error_message: "Slightly undercollateralized account",
//             },
//             GetLiquidityTestCase {
//                 cash_index: "1.1234",
//                 cash_principal: Some("123.123456"),
//                 expected_liquidity: Ok("117.055561"),
//                 test_assets: vec![
//                     TestAsset {
//                         asset: 1,
//                         ticker: "abc",
//                         decimals: 6,
//                         balance: "123.123456",
//                         price: "987.654321",
//                         liquidity_factor: "0.6789",
//                     },
//                     TestAsset {
//                         asset: 2,
//                         ticker: "def",
//                         decimals: 6,
//                         balance: "-12.123456",
//                         price: "987.654321",
//                         liquidity_factor: "0.1450",
//                     },
//                 ],
//                 error_message:
//                     "Slightly undercollateralized by assets but with some offsetting positive cash",
//             },
//         ]
//     }

//     #[test]
//     fn test_get_liquidity_all_cases() {
//         get_test_liquidity_cases()
//             .drain(..)
//             .for_each(test_get_liquidity_test_case);
//     }

//     #[test]
//     fn test_usd_value() {
//         let quantity = SignedQuantity::from_nominal(CASH, "123.123456");
//         let price = Price::from_nominal(CASH, "7.890123");
//         let actual = quantity.usd_value(price);
//         let expected = Ok(SignedQuantity::from_nominal(USD, "971.459212"));
//         assert_eq!(actual, expected);
//     }

//     #[test]
//     fn test_cash_principal_to_cash_balance_signed_quantity() {
//         let cash_principal = CashPrincipal::from_nominal("456.789012");
//         let cash_index = CashIndex::from_nominal("1.2345");

//         let actual = cash_principal_to_cash_balance_signed_quantity(cash_principal, cash_index);
//         let expected = Ok(SignedQuantity::from_nominal(CASH, "563.906035"));
//         assert_eq!(actual, expected);
//     }
// }
