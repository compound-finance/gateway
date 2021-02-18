use codec::{Decode, Encode};
use our_std::{
    convert::{TryFrom, TryInto},
    Debuggable, Deserialize, RuntimeDebug, Serialize,
};

use crate::{
    chains::{Chain, ChainAsset, Ethereum},
    rates::{InterestRateModel, ReserveFactor},
    reason::{MathError, Reason},
    symbol::{static_pow10, Symbol, Ticker, Units, CASH, USD},
    SubstrateId,
};
use num_bigint::BigUint;
use num_traits::ToPrimitive;

// Type aliases //

/// Type for representing a percentage/fractional, often between [0, 100].
pub type Bips = u128;

/// Type for representing a number of decimal places.
pub type Decimals = u8;

/// Type for a nonce.
pub type Nonce = u32;

/// Type for representing time.
pub type Timestamp = u128; // XXX u64?

/// Type of the largest possible signed integer.
pub type Int = i128;

/// Type of the largest possible unsigned integer.
pub type Uint = u128;

/// Type for a generic encoded message, potentially for any chain.
pub type EncodedNotice = Vec<u8>;

/// Type for representing an amount, potentially of any symbol.
pub type AssetAmount = Uint;

/// Type for representing an amount of CASH
pub type CashAmount = Uint;

/// Type for representing a balance of a specific asset.
pub type AssetBalance = Int;

/// Type for representing a price, potentially for any symbol.
pub type AssetPrice = Uint;

/// Type for representing an amount of an asset, together with its units.
pub type AssetQuantity = Quantity;

/// Type for representing a quantity of CASH.
pub type CashQuantity = Quantity; // ideally Quantity<{ CASH }>

/// Type for representing a quantity of USD.
pub type USDQuantity = Quantity; // ideally Quantity<{ USD }>

/// Type for a code hash.
pub type CodeHash = <Ethereum as Chain>::Hash; // XXX what to use?

/// Type for an open price feed reporter.
pub type Reporter = <Ethereum as Chain>::Address;

/// Type for a set of open price feed reporters.
#[derive(Clone, Eq, PartialEq, Encode, Decode, Default, RuntimeDebug)]
pub struct ReporterSet(pub Vec<Reporter>);

impl ReporterSet {
    pub fn contains(&self, reporter: Reporter) -> bool {
        self.0.iter().any(|e| e.as_slice() == reporter.as_slice())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> TryFrom<Vec<&'a str>> for ReporterSet {
    type Error = Reason;
    fn try_from(strings: Vec<&'a str>) -> Result<ReporterSet, Self::Error> {
        let mut reporters = Vec::with_capacity(strings.len());
        for string in strings {
            reporters.push(<Ethereum as Chain>::str_to_address(string)?)
        }
        Ok(ReporterSet(reporters))
    }
}

/// Type for enumerating sessions.
pub type SessionIndex = u32;

/// Type for an address used to identify a validator.
pub type ValidatorIdentity = <Ethereum as Chain>::Address;

/// Type for signature used to verify that a signed payload comes from a validator.
pub type ValidatorSig = <Ethereum as Chain>::Signature;

/// Type for representing the keys to sign notices.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct ValidatorKeys {
    pub substrate_id: SubstrateId,
    pub eth_address: <Ethereum as Chain>::Address,
}

/// Type for referring to either an asset or CASH.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum CashOrChainAsset {
    Cash,
    ChainAsset(ChainAsset),
}

/// LiquidityFactor for a given market.
#[derive(Serialize, Deserialize)] // used in config
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct LiquidityFactor(pub Uint);

impl From<Uint> for LiquidityFactor {
    fn from(x: u128) -> Self {
        LiquidityFactor(x)
    }
}

impl LiquidityFactor {
    pub const DECIMALS: Decimals = 4;
    pub const ZERO: LiquidityFactor = LiquidityFactor::from_nominal("0");
    pub const ONE: LiquidityFactor = LiquidityFactor::from_nominal("1");

    /// Get a liquidity factor from a string.
    /// Only for use in const contexts.
    pub const fn from_nominal(s: &'static str) -> Self {
        LiquidityFactor(uint_from_string_with_decimals(Self::DECIMALS, s))
    }
}

impl Default for LiquidityFactor {
    fn default() -> Self {
        LiquidityFactor::ZERO
    }
}

impl our_std::str::FromStr for LiquidityFactor {
    type Err = Reason;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Ok(LiquidityFactor(
            u128::from_str(string).map_err(|_| Reason::InvalidLiquidityFactor)?,
        ))
    }
}

impl From<LiquidityFactor> for String {
    fn from(string: LiquidityFactor) -> Self {
        format!("{}", string.0)
    }
}

/// Type for representing a quantity, potentially of any symbol.
#[derive(Serialize, Deserialize)] // used in config
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct AssetInfo {
    pub asset: ChainAsset,
    pub decimals: Decimals,
    pub liquidity_factor: LiquidityFactor,
    pub rate_model: InterestRateModel,
    pub reserve_factor: ReserveFactor,
    pub supply_cap: AssetAmount,
    pub symbol: Symbol,
    pub ticker: Ticker,
}

impl AssetInfo {
    pub fn minimal(asset: ChainAsset, units: Units) -> Result<Self, Reason> {
        Ok(AssetInfo {
            asset,
            decimals: units.decimals,
            liquidity_factor: LiquidityFactor::default(),
            rate_model: InterestRateModel::default(),
            reserve_factor: ReserveFactor::default(),
            supply_cap: AssetAmount::default(),
            symbol: Symbol(units.ticker.0),
            ticker: units.ticker,
        })
    }

    pub const fn units(self) -> Units {
        Units::new(self.ticker, self.decimals)
    }

    pub const fn as_balance(self, amount: AssetBalance) -> Balance {
        Balance::new(amount, self.units())
    }

    pub const fn as_balance_nominal(self, s: &'static str) -> Balance {
        Balance::from_nominal(s, self.units())
    }

    pub const fn as_quantity(self, amount: AssetAmount) -> Quantity {
        Quantity::new(amount, self.units())
    }

    pub const fn as_quantity_nominal(self, s: &'static str) -> Quantity {
        Quantity::from_nominal(s, self.units())
    }
}

// XXX ideally we should really impl Ord ourselves for these
//  and should assert ticker/units is same when comparing
//   would have to panic, though not for partial ord

/// Type for representing a price (in USD), bound to its ticker.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debuggable)]
pub struct Price {
    pub ticker: Ticker,
    pub value: AssetPrice,
}

impl Price {
    pub const DECIMALS: Decimals = USD.decimals; // Note: must be >= USD.decimals

    pub const fn new(ticker: Ticker, value: AssetPrice) -> Self {
        Price { ticker, value }
    }

    /// Get a price from a string.
    /// Only for use in const contexts.
    pub const fn from_nominal(ticker: Ticker, s: &'static str) -> Self {
        Price::new(ticker, uint_from_string_with_decimals(Self::DECIMALS, s))
    }
}

/// Type for representing a quantity of an asset, bound to its ticker and number of decimals.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debuggable)]
pub struct Quantity {
    pub value: AssetAmount,
    pub units: Units,
}

impl Quantity {
    pub const fn new(value: AssetAmount, units: Units) -> Self {
        Quantity { value, units }
    }

    /// Get a quantity from a string.
    /// Only for use in const contexts.
    pub const fn from_nominal(s: &'static str, units: Units) -> Self {
        Quantity::new(uint_from_string_with_decimals(units.decimals, s), units)
    }

    pub fn as_decrease(self) -> Result<Balance, MathError> {
        Ok(Balance::new(
            -self.value.try_into().map_err(|_| MathError::Overflow)?,
            self.units,
        ))
    }

    pub fn as_increase(self) -> Result<Balance, MathError> {
        Ok(Balance::new(
            self.value.try_into().map_err(|_| MathError::Overflow)?,
            self.units,
        ))
    }

    // Quantity<U> + Quantity<U> -> Quantity<U>
    pub fn add(self, rhs: Quantity) -> Result<Quantity, MathError> {
        if self.units != rhs.units {
            return Err(MathError::UnitsMismatch);
        }
        Ok(Quantity::new(
            self.value
                .checked_add(rhs.value)
                .ok_or(MathError::Overflow)?,
            self.units,
        ))
    }

    // Quantity<U.T> * Price<T> -> Quantity<{ USD }>
    pub fn mul_price(self, rhs: Price) -> Result<Quantity, MathError> {
        if self.units.ticker != rhs.ticker {
            return Err(MathError::UnitsMismatch);
        }
        let result = mul(
            self.value,
            self.units.decimals,
            rhs.value,
            Price::DECIMALS,
            USD.decimals,
        )?;
        Ok(Quantity::new(result as AssetAmount, USD))
    }

    // Quantity<{ USD }> / Price<T> -> Quantity<U.T>
    pub fn div_price(self, rhs: Price, units: Units) -> Result<Quantity, MathError> {
        if self.units != USD {
            return Err(MathError::PriceNotUSD);
        }
        if rhs.value == 0 {
            return Err(MathError::DivisionByZero);
        }
        if rhs.ticker != units.ticker {
            return Err(MathError::UnitsMismatch);
        }
        let result = div(
            self.value,
            self.units.decimals,
            rhs.value,
            Price::DECIMALS,
            units.decimals,
        )?;
        Ok(Quantity::new(result as AssetAmount, units))
    }

    // Quantity<U> * (CashPrincipal(+) / Quantity<U>) -> CashPrincipal(+)
    pub fn mul_cash_principal_per(
        self,
        per: CashPrincipalAmount,
    ) -> Result<CashPrincipalAmount, MathError> {
        let self_scale = 10u128.pow(self.units.decimals as u32);
        let raw = BigUint::from(self.value) * per.0 / self_scale;
        Ok(CashPrincipalAmount(
            raw.to_u128().ok_or(MathError::Overflow)?,
        ))
    }
}

/// Type for representing a signed balance of an asset, bound to its ticker and number of decimals.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct Balance {
    pub value: AssetBalance,
    pub units: Units,
}

impl Balance {
    pub const fn new(value: AssetBalance, units: Units) -> Self {
        Balance { value, units }
    }

    /// Get a quantity from a string.
    /// Only for use in const contexts.
    pub const fn from_nominal(s: &'static str, units: Units) -> Self {
        Balance::new(int_from_string_with_decimals(units.decimals, s), units)
    }

    // Balance<U> + Balance<U> -> Balance<U>
    pub fn add(self, delta: Balance) -> Result<Balance, MathError> {
        if self.units.ticker != delta.units.ticker {
            return Err(MathError::UnitsMismatch);
        }
        Ok(Balance::new(
            self.value
                .checked_add(delta.value)
                .ok_or(MathError::Overflow)?,
            self.units,
        ))
    }

    // Balance<U.T> * Price<T> -> Balance<{ USD }>
    pub fn mul_price(self, rhs: Price) -> Result<Balance, MathError> {
        if self.units.ticker != rhs.ticker {
            return Err(MathError::UnitsMismatch);
        }
        let result = mul_int(
            self.value,
            self.units.decimals,
            rhs.value.try_into().map_err(|_| MathError::Overflow)?,
            Price::DECIMALS,
            USD.decimals,
        )?;
        Ok(Balance::new(result, USD))
    }

    // Balance<U> / LiquidityFactor -> Balance<U>
    pub fn div_factor(self, rhs: LiquidityFactor) -> Result<Balance, MathError> {
        let result = div_int(
            self.value,
            self.units.decimals,
            rhs.0.try_into().map_err(|_| MathError::Overflow)?,
            LiquidityFactor::DECIMALS,
            self.units.decimals,
        )?;
        Ok(Balance::new(result, self.units))
    }

    // Balance<U> * LiquidityFactor -> Balance<U>
    pub fn mul_factor(self, rhs: LiquidityFactor) -> Result<Balance, MathError> {
        let result = mul_int(
            self.value,
            self.units.decimals,
            rhs.0.try_into().map_err(|_| MathError::Overflow)?,
            LiquidityFactor::DECIMALS,
            self.units.decimals,
        )?;
        Ok(Balance::new(result, self.units))
    }
}

/// Type for representing a balance of CASH Principal.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Default, Debuggable)]
pub struct CashPrincipal(pub AssetBalance);

impl CashPrincipal {
    pub const DECIMALS: Decimals = CASH.decimals;
    pub const ZERO: CashPrincipal = CashPrincipal(0);
    pub const ONE: CashPrincipal = CashPrincipal::from_nominal("1");

    /// Get a CASH principal balance from a string.
    /// Only for use in const contexts.
    pub const fn from_nominal(s: &'static str) -> Self {
        CashPrincipal(int_from_string_with_decimals(Self::DECIMALS, s))
    }

    pub fn add(self: Self, rhs: Self) -> Result<Self, MathError> {
        Ok(CashPrincipal(
            self.0.checked_add(rhs.0).ok_or(MathError::Overflow)?,
        ))
    }

    pub fn sub(self, rhs: Self) -> Result<Self, MathError> {
        Ok(CashPrincipal(
            self.0.checked_sub(rhs.0).ok_or(MathError::Underflow)?,
        ))
    }

    pub fn add_amount(self, amount: CashPrincipalAmount) -> Result<Self, MathError> {
        let signed = AssetBalance::try_from(amount.0).or(Err(MathError::Overflow))?;
        self.add(CashPrincipal(signed))
    }

    pub fn sub_amount(self, amount: CashPrincipalAmount) -> Result<Self, MathError> {
        let signed = AssetBalance::try_from(amount.0).or(Err(MathError::Overflow))?;
        self.sub(CashPrincipal(signed))
    }

    pub fn amount_withdrawable(self) -> Result<CashPrincipalAmount, MathError> {
        if self.0 > 0 {
            Ok(CashPrincipalAmount(self.0 as AssetAmount))
        } else {
            Ok(CashPrincipalAmount(0))
        }
    }

    pub fn amount_repayable(self) -> Result<CashPrincipalAmount, MathError> {
        if self.0 < 0 {
            Ok(CashPrincipalAmount(-self.0 as AssetAmount))
        } else {
            Ok(CashPrincipalAmount(0))
        }
    }
}

/// Type for representing an amount of CASH Principal.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Default, RuntimeDebug)]
pub struct CashPrincipalAmount(pub AssetAmount);

impl CashPrincipalAmount {
    pub const DECIMALS: Decimals = CashPrincipal::DECIMALS;
    pub const ZERO: CashPrincipalAmount = CashPrincipalAmount(0);

    /// Get a CASH principal amount from a string.
    /// Only for use in const contexts.
    pub const fn from_nominal(s: &'static str) -> Self {
        CashPrincipalAmount(uint_from_string_with_decimals(Self::DECIMALS, s))
    }

    pub fn add(self: Self, rhs: Self) -> Result<Self, MathError> {
        Ok(CashPrincipalAmount(
            self.0.checked_add(rhs.0).ok_or(MathError::Overflow)?,
        ))
    }

    pub fn sub(self, rhs: Self) -> Result<Self, MathError> {
        Ok(CashPrincipalAmount(
            self.0.checked_sub(rhs.0).ok_or(MathError::Underflow)?,
        ))
    }
}

/// Type for representing a multiplicative index on Compound Chain.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debuggable)]
pub struct CashIndex(pub Uint);

impl CashIndex {
    pub const DECIMALS: Decimals = 18;
    pub const ONE: CashIndex = CashIndex(static_pow10(Self::DECIMALS));

    /// Get a CASH index from a string.
    /// Only for use in const contexts.
    pub const fn from_nominal(s: &'static str) -> Self {
        CashIndex(uint_from_string_with_decimals(Self::DECIMALS, s))
    }

    // CashPrincipal * CashIndex -> Balance<{ CASH }>
    pub fn cash_balance(self, principal: CashPrincipal) -> Result<Balance, MathError> {
        let result = (principal.0)
            .checked_mul(self.0.try_into().map_err(|_| MathError::Overflow)?)
            .ok_or(MathError::Overflow)?;
        Ok(Balance::new(result / (CashIndex::ONE.0 as Int), CASH))
    }

    // CashPrincipal(+) * CashIndex -> Quantity<{ CASH }>
    pub fn cash_quantity(
        self,
        principal_amount: CashPrincipalAmount,
    ) -> Result<Quantity, MathError> {
        let result = (principal_amount.0)
            .checked_mul(self.0)
            .ok_or(MathError::Overflow)?;
        Ok(Quantity::new(result / CashIndex::ONE.0, CASH))
    }

    // Quantity<{ CASH }> / CashIndex -> CashPrincipal(+)
    pub fn cash_principal_amount(
        self,
        quantity: Quantity,
    ) -> Result<CashPrincipalAmount, MathError> {
        let result = (quantity.value) // XXX decimals?
            .checked_mul(CashIndex::ONE.0)
            .ok_or(MathError::Overflow)?
            .checked_div(self.0)
            .ok_or(MathError::DivisionByZero)?;
        Ok(CashPrincipalAmount(result))
    }

    /// Push the index forward by an index increment, multiplicative in the case of CashIndex
    /// New index = Old index * increment
    // XXX why is increment also an index? I think this should be its own type?
    pub fn increment(self, rhs: CashIndex) -> Result<CashIndex, MathError> {
        let result = (self.0)
            .checked_mul(rhs.0)
            .ok_or(MathError::Overflow)?
            .checked_div(Self::ONE.0)
            .ok_or(MathError::DivisionByZero)?;
        Ok(CashIndex(result))
    }
}

impl Default for CashIndex {
    fn default() -> Self {
        CashIndex::ONE
    }
}

impl<T> From<T> for CashIndex
where
    T: Into<Uint>,
{
    fn from(raw: T) -> Self {
        CashIndex(raw.into())
    }
}

/// Type for representing the additive asset indices.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct AssetIndex(pub Uint);

impl AssetIndex {
    pub const DECIMALS: Decimals = CashPrincipal::DECIMALS; // Note: decimals must match
    pub const ONE: AssetIndex = AssetIndex::from_nominal("1");

    /// Get an asset index from a string.
    /// Only for use in const contexts.
    pub const fn from_nominal(s: &'static str) -> Self {
        AssetIndex(uint_from_string_with_decimals(Self::DECIMALS, s))
    }

    pub fn cash_principal_since(
        self,
        since: AssetIndex,
        balance: AssetBalance,
    ) -> Result<CashPrincipal, MathError> {
        let delta_index = self.0.checked_sub(since.0).ok_or(MathError::Underflow)?;
        Ok(CashPrincipal(
            balance
                .checked_mul(TryFrom::try_from(delta_index).map_err(|_| MathError::Overflow)?)
                .ok_or(MathError::Overflow)?,
        ))
    }

    pub fn increment(self, amount: CashPrincipalAmount) -> Result<AssetIndex, MathError> {
        Ok(AssetIndex(
            self.0.checked_add(amount.0).ok_or(MathError::Overflow)?,
        ))
    }
}

impl Default for AssetIndex {
    fn default() -> Self {
        AssetIndex(0)
    }
}

/// A helper function for from_nominal on Quantity and Price.
///
/// Only for use in const contexts.
pub const fn uint_from_string_with_decimals(decimals: Decimals, s: &'static str) -> Uint {
    int_from_string_with_decimals(decimals, s) as Uint
}

/// Only for use in const contexts.
pub const fn int_from_string_with_decimals(decimals: Decimals, s: &'static str) -> Int {
    let bytes = s.as_bytes();
    let mut i = bytes.len();
    let mut provided_fractional_digits = 0;
    let mut past_decimal = false;
    let mut tenpow: Int = 1;
    let mut qty: Int = 0;

    // note - for loop is not allowed in `const` context
    // going from the right of the string
    loop {
        i -= 1;
        let byte = bytes[i];
        if byte == b'-' {
            if i != 0 {
                // quit, a dash somewhere it should not be
                let _should_overflow = byte + u8::max_value();
            }
            // negate
            qty *= -1;
            break;
        }

        if byte == b'.' {
            if past_decimal {
                // multiple radix - quit.
                let _should_overflow = byte + u8::max_value();
            }
            past_decimal = true;
            continue;
        }

        if !past_decimal {
            provided_fractional_digits += 1;
        }
        // will underflow whenever byte < b'0'
        let byte_as_num = byte - b'0';
        // will overflow whenever byte > b'9'
        let _should_overflow = byte + (u8::max_value() - b'9');

        qty += (byte_as_num as Int) * tenpow;
        tenpow *= 10;
        if i == 0 {
            break;
        }
    }

    if bytes.len() == 1 && past_decimal {
        // only a radix provided, quit
        let _should_overflow = bytes[0] + u8::max_value();
    }

    // never passed the radix, it is a whole number
    if !past_decimal {
        provided_fractional_digits = 0;
    }

    let number_of_zeros_to_scale_up = decimals - provided_fractional_digits;
    if number_of_zeros_to_scale_up == 0 {
        return qty;
    }

    let scalar = static_pow10(number_of_zeros_to_scale_up) as i128;
    qty * scalar
}

/// Multiply floating point numbers represented by a (value, number_of_decimals) pair and specify
/// the output number of decimals.
///
/// Not recommended to use directly, to be used in SafeMath implementations.
pub fn mul(
    a: Uint,
    a_decimals: Decimals,
    b: Uint,
    b_decimals: Decimals,
    out_decimals: Decimals,
) -> Result<Uint, MathError> {
    let all_numerator_decimals = a_decimals
        .checked_add(b_decimals)
        .ok_or(MathError::Overflow)?;
    if all_numerator_decimals > out_decimals {
        // scale down
        let scale_decimals = all_numerator_decimals
            .checked_sub(out_decimals)
            .ok_or(MathError::Underflow)?;
        let scale = 10u128
            .checked_pow(scale_decimals as u32)
            .ok_or(MathError::Overflow)?;
        Ok(a.checked_mul(b)
            .ok_or(MathError::Overflow)?
            .checked_div(scale)
            .ok_or(MathError::DivisionByZero)?)
    } else {
        // scale up
        let scale_decimals = out_decimals
            .checked_sub(all_numerator_decimals)
            .ok_or(MathError::Underflow)?;
        let scale = 10u128
            .checked_pow(scale_decimals as u32)
            .ok_or(MathError::Overflow)?;
        Ok(a.checked_mul(b)
            .ok_or(MathError::Overflow)?
            .checked_mul(scale)
            .ok_or(MathError::Overflow)?)
    }
}

/// Multiply floating point numbers represented by a (value, number_of_decimals) pair and specify
/// the output number of decimals.
///
/// Not recommended to use directly, to be used in SafeMath implementations.
pub fn mul_int(
    a: Int,
    a_decimals: u8,
    b: Int,
    b_decimals: u8,
    out_decimals: u8,
) -> Result<Int, MathError> {
    let all_numerator_decimals = a_decimals
        .checked_add(b_decimals)
        .ok_or(MathError::Overflow)?;
    if all_numerator_decimals > out_decimals {
        // scale down
        let scale_decimals = all_numerator_decimals
            .checked_sub(out_decimals)
            .ok_or(MathError::Underflow)?;
        let scale = 10i128
            .checked_pow(scale_decimals as u32)
            .ok_or(MathError::Overflow)?;
        Ok(a.checked_mul(b)
            .ok_or(MathError::Overflow)?
            .checked_div(scale)
            .ok_or(MathError::DivisionByZero)?)
    } else {
        // scale up
        let scale_decimals = out_decimals
            .checked_sub(all_numerator_decimals)
            .ok_or(MathError::Underflow)?;
        let scale = 10i128
            .checked_pow(scale_decimals as u32)
            .ok_or(MathError::Overflow)?;
        Ok(a.checked_mul(b)
            .ok_or(MathError::Overflow)?
            .checked_mul(scale)
            .ok_or(MathError::Overflow)?)
    }
}

/// Divide floating point numbers represented by a (value, number_of_decimals) pair and specify
/// the output number of decimals.
///
/// Not recommended to use directly, to be used in SafeMath implementations.
pub fn div(
    a: Uint,
    a_decimals: Decimals,
    b: Uint,
    b_decimals: Decimals,
    out_decimals: Decimals,
) -> Result<Uint, MathError> {
    let denom_decimals = b_decimals
        .checked_add(out_decimals)
        .ok_or(MathError::Overflow)?;
    if denom_decimals > a_decimals {
        // scale up
        let scale_decimals = denom_decimals
            .checked_sub(a_decimals)
            .ok_or(MathError::Underflow)?;
        let scale = 10u128
            .checked_pow(scale_decimals as u32)
            .ok_or(MathError::Overflow)?;
        Ok(a.checked_mul(scale)
            .ok_or(MathError::Overflow)?
            .checked_div(b)
            .ok_or(MathError::DivisionByZero)?)
    } else {
        // scale down
        let scale_decimals = a_decimals
            .checked_sub(denom_decimals)
            .ok_or(MathError::Underflow)?;
        let scale = 10u128
            .checked_pow(scale_decimals as u32)
            .ok_or(MathError::Overflow)?;
        Ok(a.checked_div(b)
            .ok_or(MathError::DivisionByZero)?
            .checked_div(scale)
            .ok_or(MathError::DivisionByZero)?)
    }
}

/// Divide floating point numbers represented by a (value, number_of_decimals) pair and specify
/// the output number of decimals.
///
/// Not recommended to use directly, to be used in SafeMath implementations.
pub fn div_int(
    a: Int,
    a_decimals: u8,
    b: Int,
    b_decimals: u8,
    out_decimals: u8,
) -> Result<Int, MathError> {
    let denom_decimals = b_decimals
        .checked_add(out_decimals)
        .ok_or(MathError::Overflow)?;
    if denom_decimals > a_decimals {
        // scale up
        let scale_decimals = denom_decimals
            .checked_sub(a_decimals)
            .ok_or(MathError::Underflow)?;
        let scale = 10i128
            .checked_pow(scale_decimals as u32)
            .ok_or(MathError::Overflow)?;
        Ok(a.checked_mul(scale)
            .ok_or(MathError::Overflow)?
            .checked_div(b)
            .ok_or(MathError::DivisionByZero)?)
    } else {
        // scale down
        let scale_decimals = a_decimals
            .checked_sub(denom_decimals)
            .ok_or(MathError::Underflow)?;
        let scale = 10i128
            .checked_pow(scale_decimals as u32)
            .ok_or(MathError::Overflow)?;
        Ok(a.checked_div(b)
            .ok_or(MathError::DivisionByZero)?
            .checked_div(scale)
            .ok_or(MathError::DivisionByZero)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::*;

    const ETH: Units = Units::from_ticker_str("ETH", 18);

    #[test]
    fn test_scale_codec() {
        let a = Quantity::from_nominal("3", CASH);
        let encoded = a.encode();
        let decoded = Decode::decode(&mut encoded.as_slice());
        let b = decoded.expect("value did not decode");
        assert_eq!(a, b);
    }

    #[test]
    fn test_from_nominal_with_all_decimals() {
        let a = Quantity::from_nominal("123.456789", CASH);
        let b = Quantity::new(123456789000000000000, CASH);
        assert_eq!(a, b);
    }

    #[test]
    fn test_from_nominal_with_less_than_all_decimals() {
        let a = Quantity::from_nominal("123.4", CASH);
        let b = Quantity::new(CASH.one() * 1234 / 10, CASH);
        assert_eq!(a, b);
    }

    #[test]
    fn test_from_nominal_with_no_decimals() {
        let a = Quantity::from_nominal("123", CASH);
        let b = Quantity::new(CASH.one() * 123, CASH);
        assert_eq!(a, b);
    }

    #[test]
    #[should_panic]
    fn test_from_nominal_input_string_value_out_of_range_high() {
        Quantity::from_nominal(":", CASH);
    }

    #[test]
    #[should_panic]
    fn test_from_nominal_input_string_value_out_of_range_low() {
        Quantity::from_nominal("/", CASH);
    }

    #[test]
    #[should_panic]
    fn test_from_nominal_multiple_radix() {
        Quantity::from_nominal("12.34.56", CASH);
    }

    #[test]
    #[should_panic]
    fn test_from_nominal_only_radix() {
        Quantity::from_nominal(".", CASH);
    }

    #[test]
    #[should_panic]
    fn test_from_nominal_only_radix_multiple() {
        Quantity::from_nominal("...", CASH);
    }

    #[test]
    fn test_mul_with_scale_output_equal() {
        let result = mul(2000, 3, 30000, 4, 7);
        assert_eq!(result, Ok(60000000));
    }

    #[test]
    fn test_mul_with_scale_output_up() {
        let result = mul(2000, 3, 30000, 4, 8);
        assert_eq!(result, Ok(600000000));
    }

    #[test]
    fn test_mul_with_scale_output_down() {
        let result = mul(2000, 3, 30000, 4, 6);
        assert_eq!(result, Ok(6000000));
    }

    #[test]
    fn test_div_with_scale_output_equal() {
        let result = div(2000, 3, 30000, 4, 7);
        assert_eq!(result, Ok(6666666));
    }

    #[test]
    fn test_div_with_scale_output_up() {
        let result = div(2000, 3, 30000, 4, 8);
        assert_eq!(result, Ok(66666666));
    }

    #[test]
    fn test_div_with_scale_output_down() {
        let result = div(2000, 3, 30000, 4, 6);
        assert_eq!(result, Ok(666666));
    }

    #[test]
    fn test_quantity_times_price() {
        let price = Price::from_nominal(ETH.ticker, "1500");
        let quantity = Quantity::from_nominal("5.5", ETH);
        let result = quantity.mul_price(price).unwrap();
        let expected = Quantity::from_nominal("8250", USD);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_quantity_over_price() {
        // same as example above just inverted
        let price = Price::from_nominal(ETH.ticker, "1500");
        let value = Quantity::from_nominal("8250", USD);
        let number_of_eth = value.div_price(price, ETH).unwrap();
        let expected_number_of_eth = Quantity::from_nominal("5.5", ETH);
        assert_eq!(number_of_eth, expected_number_of_eth);
    }

    #[test]
    fn test_mul_index() {
        let index = CashIndex::from_nominal("1.01");
        let principal = CashPrincipalAmount::from_nominal("100");
        let quantity = index.cash_quantity(principal).unwrap();
        let principal_ = index.cash_principal_amount(quantity).unwrap();
        assert_eq!(quantity, Quantity::from_nominal("101", CASH));
        assert_eq!(principal_, principal);
    }

    #[test]
    fn test_mul_overflow() {
        let result = mul(Uint::max_value() / 2 + 1, 0, 2, 0, 0);
        assert_eq!(result, Err(MathError::Overflow));
    }

    #[test]
    fn test_mul_overflow_boundary() {
        let result = mul(Uint::max_value(), 0, 1, 0, 0);
        assert_eq!(result, Ok(Uint::max_value()));
    }

    #[test]
    fn test_mul_overflow_boundary_2() {
        // note max value is odd thus truncated here and we lose a digit
        let result = mul(Uint::max_value() / 2, 0, 2, 0, 0);
        assert_eq!(result, Ok(Uint::max_value() - 1));
    }

    #[test]
    fn test_div_by_zero() {
        let result = div(1, 0, 0, 0, 0);
        assert_eq!(result, Err(MathError::DivisionByZero));
    }

    #[test]
    fn test_div_overflow_decimals() {
        let result = div(1, 0, 1, 0, Decimals::max_value());
        assert_eq!(result, Err(MathError::Overflow));
    }

    #[test]
    fn test_div_overflow_decimals_2() {
        let result = div(1, Decimals::max_value(), 1, 0, 0);
        assert_eq!(result, Err(MathError::Overflow));
    }

    #[test]
    fn test_cash_index_increment() {
        let old_index = CashIndex::from_nominal("1.1"); // current 10%
        let increment = CashIndex::from_nominal("1.01"); // increment by 1%
        let actual = old_index.increment(increment).unwrap();
        let expected = CashIndex::from_nominal("1.1110");
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_int_from_string_with_decimals() {
        let x = int_from_string_with_decimals(3, "-12.345");
        assert_eq!(x, -12345);
    }

    #[test]
    fn test_cash_principal_since() {
        let old_index = AssetIndex::from_nominal("1.0");
        let cur_index = AssetIndex::from_nominal("1.1");
        let balance = 100i128;
        let actual = cur_index.cash_principal_since(old_index, balance).unwrap();
        let expected = CashPrincipal::from_nominal("10");
        assert_eq!(actual, expected);
    }
}
