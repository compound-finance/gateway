use crate::symbol::static_pow10;
use crate::{
    chains::{Chain, ChainAsset, Ethereum},
    reason::{MathError, Reason},
    symbol::{Symbol, CASH, USD},
};
use codec::{Decode, Encode};
use our_std::{convert::TryFrom, Deserialize, RuntimeDebug, Serialize};

// Type aliases //

/// Type for representing a percentage/fractional, often between [0, 100].
pub type Bips = u128;

/// Type for a nonce.
pub type Nonce = u32;

/// Type for representing time.
pub type Timestamp = u128; // XXX u64?

/// Number of seconds in a year
pub const SECONDS_PER_YEAR: Timestamp = 31557600; // todo: finalize this number

/// Type of the largest possible signed integer.
pub type Int = i128;

/// Type of the largest possible unsigned integer.
pub type Uint = u128;

/// Type for a generic encoded message, potentially for any chain.
pub type EncodedNotice = Vec<u8>;

/// Type for representing an amount, potentially of any symbol.
pub type AssetAmount = Uint;

/// Type for representing a balance of a specific asset.
pub type AssetBalance = Int;

/// Type for representing a price, potentially for any symbol.
pub type AssetPrice = Uint;

/// Type for representing an amount of a specific asset.
pub type AssetQuantity = Quantity;

/// Type for representing an amount of CASH.
pub type CashQuantity = Quantity; // ideally Quantity<{ CASH }>

/// Type for representing an amount of USD.
pub type USDQuantity = Quantity; // ideally Quantity<{ USD }>

/// Type for a set of open price feed reporters.
pub type ReporterSet = Vec<<Ethereum as Chain>::Address>;

/// Type for an encoded payload within an extrinsic.
pub type SignedPayload = Vec<u8>; // XXX

/// Type for signature used to verify that a signed payload comes from a validator.
pub type ValidatorSig = [u8; 65]; // XXX secp256k1 sign, but why secp256k1?

/// Type for an address used to identify a validator.
pub type ValidatorKey = [u8; 20]; // XXX secp256k1 public key, but why secp256k1?

pub type EthAddress = ValidatorKey;

pub type SessionIndex = u32;

/// Type for a set of validator identities.
pub type ValidatorSet = Vec<ValidatorKey>; // XXX whats our set type? ordered Vec?

/// Type for representing a quantity, potentially of any symbol.
#[derive(Clone, Deserialize, Serialize)]
pub struct ConfigAsset {
    #[serde(deserialize_with = "deserialize_from_str")]
    #[serde(serialize_with = "serialize_into_str")]
    pub symbol: Symbol,

    #[serde(deserialize_with = "deserialize_from_str")]
    #[serde(serialize_with = "serialize_into_str")]
    pub asset: ChainAsset,
}

// For using in GenesisConfig / ChainSpec JSON.
// XXX
use our_std::{fmt::Display, str::FromStr};
use serde::{de, Deserializer, Serializer};

fn deserialize_from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}

fn serialize_into_str<T, S>(val: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Copy + Into<String>,
    S: Serializer,
{
    let s: String = (*val).into();
    serializer.serialize_str(&s)
}

/// Type for a set of configuration values
pub type ConfigSet<T> = Vec<T>;

/// Type for a set of configuration strings
pub type ConfigSetString = ConfigSet<String>;

/// Type for the status of an event on the queue.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum EventStatus<C: Chain> {
    Pending { signers: crate::ValidatorSet },
    Failed { hash: C::Hash, reason: Reason },
    Done,
}

// XXX ord should really assert symbol is same for these

/// Type for representing a price (in USD), bound to its symbol.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct Price(pub Symbol, pub AssetPrice);

/// Type for representing a quantity of an asset, bound to its symbol.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct Quantity(pub Symbol, pub AssetAmount);

/// Type for representing an amount of CASH Principal.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Default, RuntimeDebug)]
pub struct CashPrincipal(pub Int);

/// Type for representing the keys to sign notices
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct ChainKeys {
    pub eth_address: ValidatorKey,
}

/// Type for representing a multiplicative index on Compound Chain.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct CashIndex(pub Uint);

impl CashPrincipal {
    pub const DECIMALS: u8 = 18;

    /// Get a CASH index from a string.
    pub const fn from_nominal(s: &'static str) -> Self {
        CashPrincipal(int_from_string_with_decimals(Self::DECIMALS, s))
    }

    pub const ZERO: CashPrincipal = CashPrincipal(0);

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
}

impl CashIndex {
    pub const DECIMALS: u8 = 4;

    pub const ONE: CashIndex = CashIndex(static_pow10(Self::DECIMALS));

    /// Get a CASH index from a string.
    pub const fn from_nominal(s: &'static str) -> Self {
        CashIndex(uint_from_string_with_decimals(Self::DECIMALS, s))
    }

    // CashPrincipal(-) * CashIndex -> Quantity<{ CASH }>
    pub fn as_debt_amount(self, rhs: CashPrincipal) -> Result<Quantity, MathError> {
        if rhs.0 <= 0 {
            let result = ((-rhs.0) as AssetAmount)
                .checked_mul(self.0)
                .ok_or(MathError::Overflow)?;
            Ok(Quantity(CASH, result / CashIndex::ONE.0))
        } else {
            Err(MathError::SignMismatch)
        }
    }

    // CashPrincipal(+) * CashIndex -> Quantity<{ CASH }>
    pub fn as_hold_amount(self, rhs: CashPrincipal) -> Result<Quantity, MathError> {
        if rhs.0 >= 0 {
            let result = (rhs.0 as AssetAmount)
                .checked_mul(self.0)
                .ok_or(MathError::Overflow)?;
            Ok(Quantity(CASH, result / CashIndex::ONE.0))
        } else {
            Err(MathError::SignMismatch)
        }
    }

    // Quantity<{ CASH }> / CashIndex -> CashPrincipal(-)
    pub fn as_debt_principal(self, rhs: Quantity) -> Result<CashPrincipal, MathError> {
        let amount = rhs.1.checked_div(self.0).ok_or(MathError::DivisionByZero)?;
        let signed = -Int::try_from(amount).or(Err(MathError::Overflow))?;
        Ok(CashPrincipal(signed))
    }

    // Quantity<{ CASH }> / CashIndex -> CashPrincipal(+)
    pub fn as_hold_principal(self, rhs: Quantity) -> Result<CashPrincipal, MathError> {
        let amount = rhs.1.checked_div(self.0).ok_or(MathError::DivisionByZero)?;
        let signed = Int::try_from(amount).or(Err(MathError::Overflow))?;
        Ok(CashPrincipal(signed))
    }

    /// Push the index forward by an index increment, multiplicative in the case of CashIndex
    /// New index = Old index * increment
    pub fn increment(self, rhs: CashIndex) -> Result<CashIndex, MathError> {
        let raw = self
            .0
            .checked_mul(rhs.0)
            .ok_or(MathError::Overflow)?
            .checked_div(Self::ONE.0)
            .ok_or(MathError::DivisionByZero)?;

        Ok(CashIndex(raw))
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
    pub const DECIMALS: u8 = CashPrincipal::DECIMALS;

    /// Get an asset index from a string.
    pub const fn from_nominal(s: &'static str) -> Self {
        AssetIndex(uint_from_string_with_decimals(Self::DECIMALS, s))
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
pub const fn uint_from_string_with_decimals(decimals: u8, s: &'static str) -> Uint {
    let bytes = s.as_bytes();
    let mut i = bytes.len();
    let mut provided_fractional_digits = 0;
    let mut past_decimal = false;
    let mut tenpow: Uint = 1;
    let mut qty: Uint = 0;

    // note - for loop is not allowed in `const` context
    // going from the right of the string
    loop {
        i -= 1;
        let byte = bytes[i];
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

        qty += (byte_as_num as Uint) * tenpow;

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

    let scalar = static_pow10(number_of_zeros_to_scale_up);

    qty * scalar
}

pub const fn int_from_string_with_decimals(decimals: u8, s: &'static str) -> Int {
    let uint_response = uint_from_string_with_decimals(decimals, s);
    // note - only for use in const contexts - if uint_response > int_max then this line will overflow and stop compilation as expected
    uint_response as Int
}

impl Price {
    pub const DECIMALS: u8 = USD.decimals(); // Note: must be >= USD.decimals()

    pub const fn symbol(&self) -> Symbol {
        self.0
    }

    pub const fn value(&self) -> AssetPrice {
        self.1
    }

    /// Get a price from a string.
    pub(crate) const fn from_nominal(symbol: Symbol, s: &'static str) -> Self {
        Price(symbol, uint_from_string_with_decimals(Self::DECIMALS, s))
    }

    pub const CASH_ONE: Price = Price::from_nominal(crate::symbol::CASH, "1");
}

impl Quantity {
    pub const fn symbol(&self) -> Symbol {
        self.0
    }

    pub const fn value(&self) -> AssetAmount {
        self.1
    }

    /// Get a quantity from a string.
    pub(crate) const fn from_nominal(symbol: Symbol, s: &'static str) -> Self {
        Quantity(symbol, uint_from_string_with_decimals(symbol.decimals(), s))
    }

    /// Quantity * cash principal (per unit quantity) -> cash principal
    /// used for computing interest on interest during on_initialize
    pub fn as_cash_principal(
        self,
        cash_principal_per: CashPrincipal,
    ) -> Result<CashPrincipal, MathError> {
        let converted_cash_principal_per =
            Uint::try_from(cash_principal_per.0).map_err(|_| MathError::Overflow)?;

        let raw = mul(
            converted_cash_principal_per,
            CashPrincipal::DECIMALS,
            self.value(),
            self.symbol().decimals(),
            CashPrincipal::DECIMALS,
        )
        .ok_or(MathError::Overflow)?;

        let converted_raw = Int::try_from(raw).map_err(|_| MathError::Overflow)?;

        Ok(CashPrincipal(converted_raw))
    }
}

/// Multiply floating point numbers represented by a (value, number_of_decimals) pair and specify
/// the output number of decimals.
///
/// Not recommended to use directly, to be used in SafeMath implementations.
pub fn mul(a: Uint, a_decimals: u8, b: Uint, b_decimals: u8, out_decimals: u8) -> Option<Uint> {
    let all_numerator_decimals = a_decimals.checked_add(b_decimals)?;

    if all_numerator_decimals > out_decimals {
        // scale down
        let scale_decimals = all_numerator_decimals.checked_sub(out_decimals)?;
        let scale = 10u128.checked_pow(scale_decimals as u32)?;
        a.checked_mul(b)?.checked_div(scale)
    } else {
        // scale up
        let scale_decimals = out_decimals.checked_sub(all_numerator_decimals)?;
        let scale = 10u128.checked_pow(scale_decimals as u32)?;
        a.checked_mul(b)?.checked_mul(scale)
    }
}

/// Divide floating point numbers represented by a (value, number_of_decimals) pair and specify
/// the output number of decimals.
///
/// Not recommended to use directly, to be used in SafeMath implementations.
pub fn div(a: Uint, a_decimals: u8, b: Uint, b_decimals: u8, out_decimals: u8) -> Option<Uint> {
    let denom_decimals = b_decimals.checked_add(out_decimals)?;
    if denom_decimals > a_decimals {
        // scale up
        let scale_decimals = denom_decimals.checked_sub(a_decimals)?;
        let scale = 10u128.checked_pow(scale_decimals as u32)?;
        a.checked_mul(scale)?.checked_div(b)
    } else {
        // scale down
        let scale_decimals = a_decimals.checked_sub(denom_decimals)?;
        let scale = 10u128.checked_pow(scale_decimals as u32)?;
        a.checked_div(b)?.checked_div(scale)
    }
}

// XXX kinda sucks to import this to use?
//  and do we even want to overload types?
pub trait SafeMul<Rhs = Self> {
    /// The resulting type after multiplying
    type Output;

    /// multiply self by the "right hand side" (RHS)
    fn mul(self, rhs: Rhs) -> Result<Self::Output, MathError>;
}

// Price<S> * Quantity<S> -> Quantity<{ USD }>
impl SafeMul<Quantity> for Price {
    type Output = Quantity;

    fn mul(self, rhs: Quantity) -> Result<Self::Output, MathError> {
        if self.symbol() != rhs.symbol() {
            return Err(MathError::SymbolMismatch);
        }
        let result = mul(
            self.value(),
            Price::DECIMALS,
            rhs.value(),
            rhs.symbol().decimals(),
            USD.decimals(),
        )
        .ok_or(MathError::Overflow)?;

        Ok(Quantity(USD, result as AssetAmount))
    }
}

// Quantity<S> * Price<S> -> Quantity<{ USD }>
impl SafeMul<Price> for Quantity {
    type Output = Quantity;

    fn mul(self, rhs: Price) -> Result<Self::Output, MathError> {
        // in this case, multiplication is transitive (unlike matrix multiplication for example)
        rhs.mul(self)
    }
}

pub trait SafeDiv<Rhs = Self> {
    /// The resulting type after dividing
    type Output;

    /// divide self by the "right hand side" (RHS)
    fn div(self, rhs: Rhs) -> Result<Self::Output, MathError>;
}

// Quantity<{ USD }> / Price<S> -> Quantity<S>
impl SafeDiv<Price> for Quantity {
    type Output = Quantity;

    fn div(self, rhs: Price) -> Result<Self::Output, MathError> {
        if self.symbol() != USD {
            return Err(MathError::PriceNotUSD);
        }
        if rhs.value() == 0 {
            return Err(MathError::DivisionByZero);
        }
        let result = div(
            self.value(),
            self.symbol().decimals(),
            rhs.value(),
            Price::DECIMALS,
            rhs.symbol().decimals(),
        )
        .ok_or(MathError::Overflow)?;

        Ok(Quantity(rhs.symbol(), result as AssetAmount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::*;

    const ETH: Symbol = Symbol(
        ['E', 'T', 'H', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
        18,
    );

    #[test]
    fn test_scale_codec() {
        let a = Quantity::from_nominal(CASH, "3");
        let encoded = a.encode();
        let decoded = Decode::decode(&mut encoded.as_slice());
        let b = decoded.expect("value did not decode");
        assert_eq!(a, b);
    }

    #[test]
    fn test_from_nominal_with_all_decimals() {
        let a = Quantity::from_nominal(CASH, "123.456789");
        let b = Quantity(CASH, 123456789);
        assert_eq!(a, b);
    }

    #[test]
    fn test_from_nominal_with_less_than_all_decimals() {
        let a = Quantity::from_nominal(CASH, "123.4");
        let b = Quantity(CASH, CASH.one() * 1234 / 10);
        assert_eq!(a, b);
    }

    #[test]
    fn test_from_nominal_with_no_decimals() {
        let a = Quantity::from_nominal(CASH, "123");
        let b = Quantity(CASH, CASH.one() * 123);
        assert_eq!(a, b);
    }

    #[test]
    #[should_panic]
    fn test_from_nominal_input_string_value_out_of_range_high() {
        Quantity::from_nominal(CASH, ":");
    }

    #[test]
    #[should_panic]
    fn test_from_nominal_input_string_value_out_of_range_low() {
        Quantity::from_nominal(CASH, "/");
    }

    #[test]
    #[should_panic]
    fn test_from_nominal_multiple_radix() {
        Quantity::from_nominal(CASH, "12.34.56");
    }

    #[test]
    #[should_panic]
    fn test_from_nominal_only_radix() {
        Quantity::from_nominal(CASH, ".");
    }

    #[test]
    #[should_panic]
    fn test_from_nominal_only_radix_multiple() {
        Quantity::from_nominal(CASH, "...");
    }

    #[test]
    fn test_mul_with_scale_output_equal() {
        let result = mul(2000, 3, 30000, 4, 7);
        assert_eq!(result, Some(60000000));
    }

    #[test]
    fn test_mul_with_scale_output_up() {
        let result = mul(2000, 3, 30000, 4, 8);
        assert_eq!(result, Some(600000000));
    }

    #[test]
    fn test_mul_with_scale_output_down() {
        let result = mul(2000, 3, 30000, 4, 6);
        assert_eq!(result, Some(6000000));
    }

    #[test]
    fn test_div_with_scale_output_equal() {
        let result = div(2000, 3, 30000, 4, 7);
        assert_eq!(result, Some(6666666));
    }

    #[test]
    fn test_div_with_scale_output_up() {
        let result = div(2000, 3, 30000, 4, 8);
        assert_eq!(result, Some(66666666));
    }

    #[test]
    fn test_div_with_scale_output_down() {
        let result = div(2000, 3, 30000, 4, 6);
        assert_eq!(result, Some(666666));
    }

    #[test]
    fn test_price_times_quantity() {
        let price = Price::from_nominal(ETH, "1500");
        let quantity = Quantity::from_nominal(ETH, "5.5");
        let result = price.mul(quantity).unwrap();
        let expected = Quantity::from_nominal(USD, "8250");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_quantity_over_price() {
        // same as example above just inverted
        let price = Price::from_nominal(ETH, "1500");
        let value = Quantity::from_nominal(USD, "8250");
        let number_of_eth = value.div(price).unwrap();
        let expected_number_of_eth = Quantity::from_nominal(ETH, "5.5");
        assert_eq!(number_of_eth, expected_number_of_eth);
    }

    #[test]
    fn test_mul_index() {
        let pprincipal = CashPrincipal(100000000);
        let nprincipal = CashPrincipal(-100000000);
        let index = CashIndex::from_nominal("1.01");
        let pquantity = index.as_hold_amount(pprincipal).unwrap();
        let nquantity = index.as_debt_amount(nprincipal).unwrap();
        assert_eq!(
            index.as_debt_amount(pprincipal),
            Err(MathError::SignMismatch)
        );
        assert_eq!(pquantity, Quantity::from_nominal(CASH, "101"));
        assert_eq!(
            index.as_hold_amount(nprincipal),
            Err(MathError::SignMismatch)
        );
        assert_eq!(nquantity, Quantity::from_nominal(CASH, "101"));
    }

    #[test]
    fn test_mul_overflow() {
        let result = mul(Uint::max_value() / 2 + 1, 0, 2, 0, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_mul_overflow_boundary() {
        let result = mul(Uint::max_value(), 0, 1, 0, 0);
        assert_eq!(result, Some(Uint::max_value()));
    }

    #[test]
    fn test_mul_overflow_boundary_2() {
        // note max value is odd thus truncated here and we lose a digit
        let result = mul(Uint::max_value() / 2, 0, 2, 0, 0);
        assert_eq!(result, Some(Uint::max_value() - 1));
    }

    #[test]
    fn test_div_by_zero() {
        let result = div(1, 0, 0, 0, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_div_overflow_decimals() {
        let result = div(1, 0, 1, 0, u8::max_value());
        assert_eq!(result, None);
    }

    #[test]
    fn test_div_overflow_decimals_2() {
        let result = div(1, u8::max_value(), 1, 0, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_cash_index_increment() {
        let old_index = CashIndex::from_nominal("1.1"); // current 10%
        let increment = CashIndex::from_nominal("1.01"); // increment by 1%
        let actual = old_index.increment(increment).unwrap();
        let expected = CashIndex::from_nominal("1.1110");
        assert_eq!(actual, expected);
    }
}
