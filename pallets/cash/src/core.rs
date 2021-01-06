// Note: The substrate build requires these be re-exported.
pub use our_std::{fmt, result, result::Result};

/// Setup
use codec::{Decode, Encode};
use our_std::{
    ops::{Div, Mul},
    RuntimeDebug,
};

use crate::{
    chains::{eth, Chain, ChainId, Compound},
    notices::Notice, // XXX move here, encoding to chains
    params::MIN_TX_VALUE,
    Config,
    Module,
};

macro_rules! require {
    ($expr:expr, $reason:expr) => {
        if !$expr {
            return core::result::Result::Err($reason);
        }
    };
}

// Type aliases //

/// Type for representing an annualized rate on Compound Chain.
pub type APR = u128; // XXX custom rate type?

/// Type for a nonce.
pub type Nonce = u32;

/// Type for representing time on Compound Chain.
pub type Timestamp = u128; // XXX u64?

/// Type of the largest possible unsigned integer on Compound Chain.
pub type Uint = u128;

/// Type for a generic address, potentially on any chain.
pub type GenericAddr = Vec<u8>;

/// Type for a generic account, tied to one of the possible chains.
pub type GenericAccount = (ChainId, GenericAddr);

/// Type for a generic asset, tied to one of the possible chains.
pub type GenericAsset = (ChainId, GenericAddr);

/// Type for a generic encoded message, potentially for any chain.
pub type GenericMsg = Vec<u8>;

/// Type for a generic signature, potentially for any chain.
pub type GenericSig = Vec<u8>;

/// Type for a bunch of generic signatures.
pub type GenericSigs = Vec<GenericSig>;

/// Type for representing a price, potentially for any symbol.
pub type GenericPrice = Uint;

/// Type for representing a quantity, potentially of any symbol.
pub type GenericQty = Uint;

/// Type for an encoded payload within an extrinsic.
pub type SignedPayload = Vec<u8>; // XXX

/// Type for signature used to verify that a signed payload comes from a validator.
pub type ValidatorSig = [u8; 65]; // XXX secp256k1 sign, but why secp256k1?

/// Type for a public key used to identify a validator.
pub type ValidatorKey = [u8; 65]; // XXX secp256k1 public key, but why secp256k1?

/// Type for a set of validator identities.
pub type ValidatorSet = Vec<ValidatorKey>; // XXX whats our set type? ordered Vec?

/// Type for validator set included in the genesis config.
pub type ValidatorGenesisConfig = Vec<String>; // XXX not the right type? can we get rid altogether?

// Type definitions //

/// Type for reporting failures for reasons outside of our control.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum Reason {
    None,
    MinTxValueNotMet,
}

/// Type for the abstract symbol of an asset, not tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum Symbol {
    CASH,
    DOT,
    ETH,
    SOL,
    TEZ,
    USD,
    USDC,
}

impl Symbol {
    pub const fn decimals(self) -> u8 {
        match self {
            Symbol::CASH => 6,
            Symbol::DOT => 10,
            Symbol::ETH => 18,
            Symbol::SOL => 18, // XXX ?
            Symbol::TEZ => 6,
            Symbol::USD => 6,
            Symbol::USDC => 6,
        }
    }
}

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
pub enum NoticeStatus<C: Chain> {
    Missing,
    Pending {
        signers: crate::ValidatorSet,
        signatures: GenericSigs,
        notice: Notice<C>,
    },
    Done,
}

/// XXX doc
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Account<C: Chain>(pub C::Address);

/// XXX doc
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Asset<C: Chain>(pub C::Address);

impl<C: Chain> From<Asset<C>> for GenericAsset {
    fn from(asset: Asset<C>) -> Self {
        (C::ID, asset.0.into())
    }
}

/// XXX doc
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price<const S: Symbol>(pub GenericPrice);

/// XXX doc
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quantity<const S: Symbol>(pub GenericQty);

/// Type for representing a balance index on Compound Chain.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct Index(pub Uint);

impl<const S: Symbol> Price<S> {
    pub const fn from_nominal(nominal: f64) -> Self {
        Price::<S>((nominal as Uint) * 10 ^ (S.decimals() as Uint))
    }
}

impl<const S: Symbol> Quantity<S> {
    pub const fn from_nominal(nominal: f64) -> Self {
        Quantity::<S>((nominal as Uint) * 10 ^ (S.decimals() as Uint))
    }
}

impl Default for Index {
    fn default() -> Self {
        Index(1) // XXX do we need more 'precision' for ONE?
    }
}

impl<const S: Symbol, T> From<T> for Price<S>
where
    T: Into<GenericPrice>,
{
    fn from(raw: T) -> Self {
        Price(raw.into())
    }
}

impl<const S: Symbol, T> From<T> for Quantity<S>
where
    T: Into<GenericQty>,
{
    fn from(raw: T) -> Self {
        Quantity(raw.into())
    }
}

impl<T> From<T> for Index
where
    T: Into<Uint>,
{
    fn from(raw: T) -> Self {
        Index(raw.into())
    }
}

// Price<S> * Quantity<S> -> Quantity<{ USD }>
impl<const S: Symbol> Mul<Quantity<S>> for Price<S> {
    type Output = Quantity<{ Symbol::USD }>;

    fn mul(self, rhs: Quantity<S>) -> Self::Output {
        Quantity(self.0 * rhs.0)
    }
}

// Quantity<S> * Price<S> -> Quantity<S>
impl<const S: Symbol> Mul<Price<S>> for Quantity<S> {
    type Output = Quantity<{ Symbol::USD }>;

    fn mul(self, rhs: Price<S>) -> Self::Output {
        Quantity(self.0 * rhs.0)
    }
}

// Quantity<{ USD }> / Price<S> -> Quantity<S>
impl<const S: Symbol> Div<Price<S>> for Quantity<{ Symbol::USD }> {
    type Output = Quantity<{ S }>;

    fn div(self, rhs: Price<S>) -> Self::Output {
        Quantity(self.0 / rhs.0)
    }
}

// Quantity<{ USD }> / Quantity<S> -> Price<S>
impl<const S: Symbol> Div<Quantity<S>> for Quantity<{ Symbol::USD }> {
    type Output = Price<S>;

    fn div(self, rhs: Quantity<S>) -> Self::Output {
        Price(self.0 / rhs.0)
    }
}

// Quantity<S> * Index -> Quantity<S>
impl<const S: Symbol> Mul<Index> for Quantity<S> {
    type Output = Quantity<S>;

    fn mul(self, rhs: Index) -> Self::Output {
        Quantity(self.0 * rhs.0)
    }
}

// Helper functions //

// pub fn price<T: Config, const S: Symbol>() -> Price<S> {
//     match S {
//         Symbol::CASH => Price::from_nominal(1.0),
//         _ => Price(<Module<T>>::prices(S)),
//     }
// }

// Protocol interface //

pub fn apply_eth_event_internal(event: eth::Event) -> Result<eth::Event, Reason> {
    Ok(event) // XXX
}

// pub fn extract_internal<T: Config, C: Chain, const S: Symbol>(
//     asset: Asset<C>,
//     holder: Account<Compound>,
//     recipient: Account<C>,
//     principal: Quantity<S>,
// ) -> Result<(), Reason> {
//     // Require Recipient.Chain=Asset.Chain XXX proven by compiler
//     let supply_index = <Module<T>>::supply_index(Into::<GenericAsset>::into(asset));
//     let amount = principal * supply_index;
//     let value = amount * price::<T, S>();
//     require!(value >= MIN_TX_VALUE, Reason::MinTxValueNotMet);
//
//     // Read Require HasLiquidityToReduceCollateralAsset(Holder, Asset, Amount)
//     // ReadsCashBorrowPrincipalBorrower, CashCostIndexPair, CashYield, CashSpread, Price*, SupplyPrincipal*, Borrower, StabilityFactor*
//     // Read TotalSupplyNew=TotalSupplyPrincipalAsset-Principal
//     // Underflow: Not enough total funds to extract ${Amount}
//     // Read HolderSupplyNew=SupplyPrincipalAsset, Holder-Principal
//     // Underflow: ${Holder} does not have enough funds to extract ${Amount}
//     // Set TotalSupplyPrincipalAsset=TotalSupplyNew
//     // Set SupplyPrincipalAsset, Holder=HolderSupplyNew
//     // Add ExtractionNotice(Asset, Recipient, Amount) to NoticeQueueRecipient.Chain
//     Ok(()) // XXX
// }
//
// // XXX should we expect amounts are already converted to our bigint type here?
// //  probably not, probably inputs should always be fixed width?
// //   actually now I think we can always guarantee to parse ascii numbers in lisp requests into bigints
// pub fn extract_cash_principal_internal<T: Config, C: Chain>(
//     holder: Account<Compound>,
//     recipient: Account<C>,
//     principal: Quantity<{ Symbol::CASH }>,
// ) -> Result<(), Reason> {
//     let yield_index = <Module<T>>::cash_yield_index();
//     let amount = principal * yield_index;
//     let value = amount * price::<T, { Symbol::CASH }>();
//     require!(value >= MIN_TX_VALUE, Reason::MinTxValueNotMet);
//
//     // Note: we do not check health here, since CASH cannot be borrowed against yet.
//     // let chain_cash_hold_principal_new = <Module<T>>::chain_cash_hold_principal(recipient.chain) + amount_principal;
//     // let holder_cash_hold_principal_new = <Module<T>>::cash_hold_principal(holder) - amount_principal;
//     // XXX Underflow: ${Account} does not have enough CASH to extract ${Amount};
//     // <Module<T>>::ChainCashHoldPrincipal::insert(recipient, chain_cash_hold_principal_new);
//     // <Module<T>>::C
//     //     Set TotalCashHoldPrincipalRecipient.Chain=ChainCashHoldPrincipalNew;
//     //     Set CashHoldPrincipalHolder=HolderCashHoldPrincipalNew;
//     //     Add CashExtractionNotice(Recipient, Amount, YieldIndex) to NoticeQueueRecipient.Chain;
//     Ok(()) // XXX
// }

#[cfg(test)]
mod tests {
    // XXX
}
