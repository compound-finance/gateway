use crate::{
  chains::{Chain, ChainId, Ethereum},
  notices::Notice,
  symbol::{Symbol, CASH, NIL, USD},
  Config, Module,
};
use codec::{Decode, Encode};
use our_std::{
  ops::{Div, Mul},
  RuntimeDebug,
};

// Type aliases //

/// Type for representing an annualized rate on Compound Chain.
pub type APR = u128; // XXX custom rate type?

/// Type for a nonce.
pub type Nonce = u32;

/// Type for representing time on Compound Chain.
pub type Timestamp = u128; // XXX u64?

/// Type of the largest possible unsigned integer on Compound Chain.
pub type Uint = u128;

/// Type for a generic encoded message, potentially for any chain.
pub type EncodedNotice = Vec<u8>;

/// Type for representing a price, potentially for any symbol.
pub type AssetPrice = Uint;

/// Type for representing an amount of any asset
pub type AssetAmount = Uint;

/// Type for representing an amount of Cash
pub type CashAmount = Uint;

/// Type for representing a quantity, potentially of any symbol.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum Maxable<T> {
  Max,
  Value(T),
}

impl<T> From<T> for Maxable<T> {
  fn from(t: T) -> Self {
    Maxable::Value(t)
  }
}

pub fn get_max_value<T>(max: Maxable<T>, when_max: &dyn Fn() -> T) -> T {
  match max {
    Maxable::Max => when_max(),
    Maxable::Value(t) => t,
  }
}

/// Type for chain signatures
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainSignature {
  Eth(<Ethereum as Chain>::Signature),
}

/// Type for a list of chain signatures
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainSignatureList {
  Eth(Vec<<Ethereum as Chain>::Signature>),
}

/// Type for chain accounts
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainAccount {
  Eth(<Ethereum as Chain>::Address),
}

/// Type for chain assets
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainAsset {
  Eth(<Ethereum as Chain>::Address),
}

/// Type for a set of open price feed reporters.
pub type ReporterSet = Vec<<Ethereum as Chain>::Address>;

/// Type for an encoded payload within an extrinsic.
pub type SignedPayload = Vec<u8>; // XXX

/// Type for signature used to verify that a signed payload comes from a validator.
pub type ValidatorSig = [u8; 65]; // XXX secp256k1 sign, but why secp256k1?

/// Type for an address used to identify a validator.
pub type ValidatorKey = [u8; 20]; // XXX secp256k1 public key, but why secp256k1?

/// Type for a set of validator identities.
pub type ValidatorSet = Vec<ValidatorKey>; // XXX whats our set type? ordered Vec?

/// Type for a set of configuration values
pub type ConfigSet<T> = Vec<T>;

/// Type for the status of an event on the queue.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum EventStatus<C: Chain> {
  Pending {
    signers: crate::ValidatorSet,
  },
  Failed {
    hash: C::Hash,
    reason: crate::Reason,
  },
  Done,
}

/// Type for the status of a notice on the queue.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum NoticeStatus {
  Missing,
  Pending {
    signers: crate::ValidatorSet,
    signatures: ChainSignatureList,
    notice: Notice,
  },
  Done,
}

/// Type for representing a price (in USD), bound to its symbol.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct Price(pub Symbol, pub AssetPrice);

/// Type for representing a quantity of an asset, bound to its symbol.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct Quantity(pub Symbol, pub AssetAmount);

/// Type for representing a multiplicative index on Compound Chain.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct MulIndex(pub Uint);

impl Default for MulIndex {
  fn default() -> Self {
    MulIndex(1) // XXX do we need more 'precision' for ONE?
  }
}

impl<T> From<T> for MulIndex
where
  T: Into<Uint>,
{
  fn from(raw: T) -> Self {
    MulIndex(raw.into())
  }
}

impl Price {
  pub const DECIMALS: u8 = USD.decimals(); // Note: must be >= USD.decimals()

  pub const fn symbol(&self) -> Symbol {
    self.0
  }

  pub const fn amount(&self) -> AssetPrice {
    self.1
  }

  pub const fn from_nominal(symbol: Symbol, nominal: f64) -> Self {
    Price(symbol, (nominal * pow10(Self::DECIMALS)) as Uint)
  }

  pub const fn to_nominal(&self) -> f64 {
    (self.amount() as f64) / pow10(self.symbol().decimals())
  }
}

impl Quantity {
  pub const fn symbol(&self) -> Symbol {
    self.0
  }

  pub const fn amount(&self) -> AssetAmount {
    self.1
  }

  pub const fn from_nominal(symbol: Symbol, nominal: f64) -> Self {
    Quantity(symbol, (nominal * pow10(symbol.decimals())) as Uint)
  }

  pub const fn to_nominal(&self) -> f64 {
    (self.amount() as f64) / pow10(self.symbol().decimals())
  }
}

// Price<S> * Quantity<S> -> Quantity<{ USD }>
impl Mul<Quantity> for Price {
  type Output = Quantity;

  fn mul(self, rhs: Quantity) -> Self::Output {
    assert!(
      self.symbol() == rhs.symbol(),
      "can only multiply a price and quantity for the same symbol"
    );
    Quantity(
      USD,
      self.amount() * rhs.amount()
        / (pow10(Price::DECIMALS + rhs.symbol().decimals() - USD.decimals()) as AssetAmount),
    )
  }
}

// Quantity<S> * Price<S> -> Quantity<{ USD }>
impl Mul<Price> for Quantity {
  type Output = Quantity;

  fn mul(self, rhs: Price) -> Self::Output {
    assert!(
      self.symbol() == rhs.symbol(),
      "can only multiply a quantity and price for the same symbol"
    );
    Quantity(
      USD,
      self.amount() * rhs.amount()
        / (pow10(Price::DECIMALS + self.symbol().decimals() - USD.decimals()) as AssetAmount),
    )
  }
}

// Quantity<{ USD }> / Price<S> -> Quantity<S>
impl Div<Price> for Quantity {
  type Output = Quantity;

  fn div(self, rhs: Price) -> Self::Output {
    assert!(
      self.symbol() == USD,
      "division by price defined only for USD quantities"
    );
    assert!(rhs.amount() > 0, "division by price not greater than zero");
    Quantity(
      rhs.symbol(),
      self.amount()
        * (pow10(Price::DECIMALS + rhs.symbol().decimals() - USD.decimals()) as AssetPrice)
        / rhs.amount(),
    )
  }
}

// Quantity<{ USD }> / Quantity<S> -> Price<S>
impl Div<Quantity> for Quantity {
  type Output = Price;

  fn div(self, rhs: Quantity) -> Self::Output {
    assert!(
      self.symbol() == USD,
      "division by quantity defined only for USD quantities"
    );
    assert!(
      rhs.amount() > 0,
      "division by quantity not greater than zero"
    );
    Price(
      rhs.symbol(),
      self.amount()
        * (pow10(Price::DECIMALS + rhs.symbol().decimals() - USD.decimals()) as AssetAmount)
        / rhs.amount(),
    )
  }
}

// Quantity<S> * MulIndex -> Quantity<S>
impl Mul<MulIndex> for Quantity {
  type Output = Quantity;

  fn mul(self, rhs: MulIndex) -> Self::Output {
    Quantity(self.symbol(), self.amount() * rhs.0)
  }
}

// Helper functions //

pub const fn pow10(decimals: u8) -> f64 {
  let mut i = 0;
  let mut v = 10.0;
  loop {
    i += 1;
    if i >= decimals {
      return v;
    }
    v *= 10.0;
  }
}

pub fn price<T: Config>(symbol: Symbol) -> Price {
  match symbol {
    CASH => Price::from_nominal(CASH, 1.0),
    _ => Price(symbol, <Module<T>>::prices(symbol)),
  }
}

pub fn symbol<T: Config>(asset: ChainAsset) -> Symbol {
  // XXX lookup in storage
  Symbol(
    ['E', 'T', 'H', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
    18,
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  const ETH: Symbol = Symbol(
    ['E', 'T', 'H', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
    18,
  );

  #[test]
  fn test_one() {
    let a = Quantity(CASH, 1000000);
    let b = Quantity::from_nominal(CASH, 1.0);
    let c = b.to_nominal();
    assert_eq!(a, b);
    assert_eq!(c, 1.0);
  }

  #[test]
  fn test_mul_qp() {
    // Quantity<S> * Price<S> -> Quantity<USD>
    // Price<S> * Quantity<S> -> Quantity<USD>
    let q = Quantity::from_nominal(CASH, 1.0);
    let p = Price::from_nominal(CASH, 2.0);
    assert_eq!(q * p, Quantity::from_nominal(USD, 2.0));
    assert_eq!(p * q, Quantity::from_nominal(USD, 2.0));
  }

  #[test]
  #[should_panic(expected = "can only multiply a quantity and price for the same symbol")]
  fn test_mul_qp_error() {
    let _ = Quantity::from_nominal(ETH, 1.0) * Price::from_nominal(CASH, 2.0);
  }

  #[test]
  #[should_panic(expected = "can only multiply a price and quantity for the same symbol")]
  fn test_mul_pq_error() {
    let _ = Price::from_nominal(CASH, 2.0) * Quantity::from_nominal(ETH, 1.0);
  }

  #[test]
  fn test_div_qp() {
    // Quantity<{ USD }> / Price<S> -> Quantity<S>
    let q = Quantity::from_nominal(USD, 365.0);
    let p = Price::from_nominal(ETH, 10.0);
    assert_eq!(q / p, Quantity::from_nominal(ETH, 36.5));
  }

  #[test]
  #[should_panic(expected = "division by price defined only for USD quantities")]
  fn test_div_qp_error() {
    let _ = Quantity::from_nominal(CASH, 2.0) / Price::from_nominal(ETH, 1.0);
  }

  #[test]
  #[should_panic(expected = "division by price not greater than zero")]
  fn test_div_qp_div_zero() {
    let _ = Quantity::from_nominal(USD, 2.0) / Price::from_nominal(ETH, 0.0);
  }

  #[test]
  fn test_div_qq() {
    // Quantity<{ USD }> / Quantity<S> -> Price<S>
    let q = Quantity::from_nominal(USD, 10.0);
    let u = Quantity::from_nominal(ETH, 3.0);
    assert_eq!(q / u, Price::from_nominal(ETH, 3.33333333333));
  }

  #[test]
  #[should_panic(expected = "division by quantity defined only for USD quantities")]
  fn test_div_qq_error() {
    let _ = Quantity::from_nominal(CASH, 2.0) / Quantity::from_nominal(ETH, 1.0);
  }

  #[test]
  #[should_panic(expected = "division by quantity not greater than zero")]
  fn test_div_qq_div_zero() {
    let _ = Quantity::from_nominal(USD, 2.0) / Quantity::from_nominal(ETH, 0.0);
  }

  #[test]
  fn test_scale_codec() {
    let a = Quantity::from_nominal(CASH, 3.0);
    let encoded = a.encode();
    dbg!(encoded.clone()); // XXX
    let decoded = Decode::decode(&mut encoded.as_slice());
    let b = decoded.expect("value did not decode");
    dbg!(u128::MAX); // XXX
    assert_eq!(a, b);
  }
}
