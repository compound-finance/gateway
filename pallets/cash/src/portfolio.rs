use crate::chains::{ChainAccount, ChainAsset};
use crate::core::{price, symbol};
use crate::reason::{MathError, Reason};
use crate::symbol::{Symbol, CASH, USD};
use crate::types::{
    div_int, int_from_string_with_decimals, mul_int, AssetAmount, AssetBalance, CashAmount,
    CashIndex, CashPrincipal, Int, LiquidityFactor, Price, Quantity,
};
use crate::{
    AssetBalances, AssetsWithNonZeroBalance, CashPrincipals, GlobalCashIndex, LiquidityFactors,
};
use codec::{Decode, Encode};
use frame_support::storage::{StorageDoubleMap, StorageMap, StorageValue};
use our_std::convert::TryInto;
use our_std::RuntimeDebug;

/// Type for representing a signed quantity of an asset, bound to its symbol.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct SignedQuantity(pub Symbol, pub AssetBalance);

type USDSignedQuantity = SignedQuantity;

impl SignedQuantity {
    pub const fn symbol(&self) -> Symbol {
        self.0
    }

    pub const fn value(&self) -> AssetBalance {
        self.1
    }

    /// Get a quantity from a string.
    pub const fn from_nominal(symbol: Symbol, s: &'static str) -> Self {
        SignedQuantity(symbol, int_from_string_with_decimals(symbol.decimals(), s))
    }

    pub fn from_asset_and_balance(
        asset: &ChainAsset,
        amount: AssetBalance,
    ) -> Result<Self, Reason> {
        let symbol = symbol(asset).ok_or(Reason::InvalidSymbol)?;
        Ok(SignedQuantity(symbol, amount))
    }

    pub fn from_asset_and_amount(asset: &ChainAsset, amount: AssetAmount) -> Result<Self, Reason> {
        let symbol = symbol(asset).ok_or(Reason::InvalidSymbol)?;
        let balance: AssetBalance = amount.try_into().map_err(|_| MathError::Overflow)?;
        Ok(SignedQuantity(symbol, balance))
    }

    /// Negate the quantity
    pub fn negate(self) -> Self {
        SignedQuantity(self.symbol(), self.value() * -1)
    }

    pub fn add(self, rhs: SignedQuantity) -> Result<Self, Reason> {
        if self.symbol() != rhs.symbol() {
            return Err(Reason::InvalidSymbol);
        }

        let raw = self
            .value()
            .checked_add(rhs.value())
            .ok_or(MathError::Overflow)?;

        Ok(SignedQuantity(self.symbol(), raw))
    }

    pub fn usd_value(self, price: Price) -> Result<USDSignedQuantity, Reason> {
        if self.symbol() != price.symbol() {
            return Err(Reason::InvalidSymbol);
        }
        let converted_price: AssetBalance =
            price.value().try_into().map_err(|_| MathError::Overflow)?;

        let raw = mul_int(
            self.value(),
            self.symbol().decimals(),
            converted_price,
            Price::DECIMALS,
            USD.decimals(),
        )
        .ok_or(MathError::Overflow)?;

        Ok(SignedQuantity(USD, raw))
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Portfolio {
    pub positions: Vec<SignedQuantity>,
    pub cash_principal: CashPrincipal,
}

fn cash_balance_to_principal(balance: CashAmount) -> Result<CashPrincipal, Reason> {
    let index: CashIndex = GlobalCashIndex::get();
    Ok(index.as_hold_principal(Quantity(CASH, balance))?)
}

fn cash_principal_to_cash_balance_signed_quantity(
    cash_principal: CashPrincipal,
    cash_index: CashIndex,
) -> Result<SignedQuantity, Reason> {
    let converted_cash_index: Int = cash_index.0.try_into().map_err(|_| MathError::Overflow)?;
    let raw = mul_int(
        cash_principal.0,
        CashPrincipal::DECIMALS,
        converted_cash_index,
        CashIndex::DECIMALS,
        CASH.decimals(),
    )
    .ok_or(MathError::Overflow)?;

    Ok(SignedQuantity(CASH, raw))
}

impl Portfolio {
    /// Read a portfolio from storage
    pub fn from_storage(account: &ChainAccount) -> Result<Portfolio, Reason> {
        let cash_principal: CashPrincipal = CashPrincipals::get(account);

        let assets_with_non_zero_balance = AssetsWithNonZeroBalance::get(account);

        let mut positions = Vec::new();

        for asset in assets_with_non_zero_balance {
            let asset_balance: AssetBalance = AssetBalances::get(asset, account);
            let symbol = crate::core::symbol(&asset).ok_or(Reason::AssetSymbolNotFound)?;
            let signed_quantity = SignedQuantity(symbol, asset_balance);
            positions.push(signed_quantity);
        }

        Ok(Portfolio {
            positions,
            cash_principal,
        })
    }

    pub fn asset_balance_decrease(
        self,
        asset: &ChainAsset,
        decrease_in_balance: AssetAmount,
    ) -> Result<Portfolio, Reason> {
        let change_in_balance =
            SignedQuantity::from_asset_and_amount(asset, decrease_in_balance)?.negate();
        self.asset_balance_change(change_in_balance)
    }

    pub fn asset_balance_increase(
        self,
        asset: &ChainAsset,
        increase_in_balance: AssetAmount,
    ) -> Result<Portfolio, Reason> {
        let change_in_balance = SignedQuantity::from_asset_and_amount(asset, increase_in_balance)?;
        self.asset_balance_change(change_in_balance)
    }

    /// Modify an asset balance to see what it does to account liquidity
    fn asset_balance_change(self, change_in_balance: SignedQuantity) -> Result<Portfolio, Reason> {
        let mut positions = Vec::new();
        let mut seen: bool = false;

        for current_quantity in self.positions {
            let new_quantity = if current_quantity.symbol() == change_in_balance.symbol() {
                seen = true;
                current_quantity.add(change_in_balance)?
            } else {
                current_quantity
            };
            positions.push(new_quantity);
        }

        if !seen {
            positions.push(change_in_balance);
        }

        Ok(Portfolio {
            positions,
            cash_principal: self.cash_principal,
        })
    }

    pub fn cash_balance_increase(self, cash_balance: CashAmount) -> Result<Portfolio, Reason> {
        let cash_principal = cash_balance_to_principal(cash_balance)?;
        self.cash_principal_change(cash_principal)
    }

    pub fn cash_balance_decrease(self, cash_balance: CashAmount) -> Result<Portfolio, Reason> {
        let cash_principal = cash_balance_to_principal(cash_balance)?.negate();
        self.cash_principal_change(cash_principal)
    }

    /// Modify the cash principal to see what it does to account liquidity
    pub fn cash_principal_change(
        self,
        change_in_principal: CashPrincipal,
    ) -> Result<Portfolio, Reason> {
        let new_cash_principal = self.cash_principal.add(change_in_principal)?;
        Ok(Portfolio {
            positions: self.positions,
            cash_principal: new_cash_principal,
        })
    }

    /// Read the cash principal as a balance, accesses storage to read the cash index
    fn get_cash_balance(self: &Self) -> Result<USDSignedQuantity, Reason> {
        let cash_price = price(CASH);
        let cash_principal = self.cash_principal;
        // XXX this read clashes somewhat with the rest of the code due to the impure nature
        // consider parameterizing or pulling into portfolio struct for more control
        let cash_index: CashIndex = GlobalCashIndex::get();
        let cash_balance =
            cash_principal_to_cash_balance_signed_quantity(cash_principal, cash_index)?;

        cash_balance.usd_value(cash_price)
    }

    /// Get this accounts liquidity value
    pub fn get_liquidity(self: &Self) -> Result<USDSignedQuantity, Reason> {
        let mut liquidity = self.get_cash_balance()?;

        for position in self.positions.iter() {
            let price = price(position.symbol());
            let liquidity_factor: LiquidityFactor = LiquidityFactors::get(&position.symbol());
            let value = position.usd_value(price)?;
            let converted_liquidation_factor: AssetBalance = liquidity_factor
                .0
                .try_into()
                .map_err(|_| MathError::Overflow)?;

            let liquidity_value = if value.value() > 0 {
                let raw = mul_int(
                    value.value(),
                    value.symbol().decimals(),
                    converted_liquidation_factor,
                    LiquidityFactor::DECIMALS,
                    value.symbol().decimals(),
                )
                .ok_or(MathError::Overflow)?;

                SignedQuantity(value.symbol(), raw)
            } else {
                let raw = div_int(
                    value.value(),
                    value.symbol().decimals(),
                    converted_liquidation_factor,
                    LiquidityFactor::DECIMALS,
                    value.symbol().decimals(),
                )?;

                SignedQuantity(value.symbol(), raw)
            };

            liquidity = liquidity.add(liquidity_value)?;
        }

        Ok(liquidity)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::chains::ChainAsset;
    use crate::mock::*;
    use crate::types::{CashIndex, LiquidityFactor, Quantity};
    use crate::{AssetSymbols, Prices};

    #[test]
    fn test_from_storage() {
        new_test_ext().execute_with(|| {
            let asset1 = ChainAsset::Eth([238; 20]);
            let asset2 = ChainAsset::Eth([100; 20]);
            let account = ChainAccount::Eth([0; 20]);
            let asset1_symbol = Symbol::new("ETH", 18);
            let asset2_symbol = Symbol::new("COMP", 6);
            let asset1_balance =
                Quantity::from_nominal(asset1_symbol, "11.123456789").1 as AssetBalance;
            let cash_principal = CashPrincipal::from_nominal("100");

            // could also set CashPrincipals
            Prices::insert(asset1_symbol, 1451_123456); // $1451.123456
            Prices::insert(asset2_symbol, 500_789123); // $1451.123456
            AssetSymbols::insert(&asset1, asset1_symbol);
            AssetSymbols::insert(&asset2, asset2_symbol);
            AssetsWithNonZeroBalance::insert(account.clone(), vec![asset1.clone(), asset2.clone()]);

            AssetBalances::insert(asset1.clone(), account.clone(), asset1_balance);
            CashPrincipals::insert(&account, cash_principal);
            // GlobalCashIndex::put(CashIndex::from_nominal("1.1234"));
            // LiquidityFactors::insert(&asset1, LiquidationFactor::from_nominal("0.432"));
            // LiquidityFactors::insert(&asset2, LiquidationFactor::from_nominal("0.543"));

            let expected = Portfolio {
                positions: vec![
                    SignedQuantity(asset1_symbol, asset1_balance),
                    SignedQuantity(asset2_symbol, 0),
                ],
                cash_principal: cash_principal,
            };

            let actual = Portfolio::from_storage(&account).unwrap();
            assert_eq!(actual, expected);

            // test seen
            let change_in_balance = SignedQuantity(asset1_symbol, asset1_balance);
            let actual = actual.asset_balance_change(change_in_balance).unwrap();
            let expected = Portfolio {
                positions: vec![
                    SignedQuantity(asset1_symbol, asset1_balance * 2),
                    SignedQuantity(asset2_symbol, 0),
                ],
                cash_principal: cash_principal,
            };
            assert_eq!(actual, expected);

            // test not seen
            let asset3_symbol = Symbol::new("FOO", 6);
            let asset3_balance = SignedQuantity(asset3_symbol, 1234567);
            let actual = actual.asset_balance_change(asset3_balance).unwrap();
            let expected = Portfolio {
                positions: vec![
                    SignedQuantity(asset1_symbol, asset1_balance * 2),
                    SignedQuantity(asset2_symbol, 0),
                    asset3_balance,
                ],
                cash_principal: cash_principal,
            };
            assert_eq!(actual, expected);

            // test modify cash
            let change_in_principal = CashPrincipal::from_nominal("123456");
            let actual = actual.cash_principal_change(change_in_principal).unwrap();
            let expected = Portfolio {
                positions: vec![
                    SignedQuantity(asset1_symbol, asset1_balance * 2),
                    SignedQuantity(asset2_symbol, 0),
                    asset3_balance,
                ],
                cash_principal: cash_principal.add(change_in_principal).unwrap(),
            };
            assert_eq!(actual, expected);
        });
    }

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
            let mut assets_with_nonzero_balances = Vec::new();

            for asset_case in case.test_assets {
                let asset = ChainAsset::Eth([asset_case.asset; 20]);
                assets_with_nonzero_balances.push(asset);

                let symbol = Symbol::new(&asset_case.ticker, asset_case.decimals);
                let price = Price::from_nominal(symbol, asset_case.price);

                AssetSymbols::insert(&asset, symbol);
                Prices::insert(&symbol, price.value());
                let balance = SignedQuantity::from_nominal(symbol, asset_case.balance);
                AssetBalances::insert(&asset, &account, balance.value());
                LiquidityFactors::insert(
                    &symbol,
                    LiquidityFactor::from_nominal(asset_case.liquidity_factor),
                )
            }
            AssetsWithNonZeroBalance::insert(&account, assets_with_nonzero_balances);

            GlobalCashIndex::put(CashIndex::from_nominal(case.cash_index));
            if let Some(cash_principal) = case.cash_principal {
                CashPrincipals::insert(&account, CashPrincipal::from_nominal(cash_principal));
            }

            let actual = Portfolio::from_storage(&account).unwrap().get_liquidity();
            let expected = match case.expected_liquidity {
                Ok(liquidity_str) => Ok(SignedQuantity::from_nominal(USD, liquidity_str)),
                Err(e) => Err(e),
            };

            assert_eq!(expected, actual, "{}", case.error_message);
        });
    }

    fn get_test_liquidity_cases() -> Vec<GetLiquidityTestCase> {
        // todo: xxx finish these test cases
        vec![
            /*GetLiquidityTestCase {
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
            },*/
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

    #[test]
    fn test_usd_value() {
        let quantity = SignedQuantity::from_nominal(CASH, "123.123456");
        let price = Price::from_nominal(CASH, "7.890123");
        let actual = quantity.usd_value(price);
        let expected = Ok(SignedQuantity::from_nominal(USD, "971.459212"));
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cash_principal_to_cash_balance_signed_quantity() {
        let cash_principal = CashPrincipal::from_nominal("456.789012");
        let cash_index = CashIndex::from_nominal("1.2345");

        let actual = cash_principal_to_cash_balance_signed_quantity(cash_principal, cash_index);
        let expected = Ok(SignedQuantity::from_nominal(CASH, "563.906035"));
        assert_eq!(actual, expected);
    }
}
