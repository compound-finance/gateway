// Note: The substrate build requires these be re-exported.
pub use our_std::{fmt, result, result::Result};

/// Setup
use codec::{Decode, Encode};
use our_std::{
    ops::{Div, Mul},
    RuntimeDebug,
};

use crate::{
    chains::{eth, Chain, Compound},
    params::MIN_TX_VALUE,
    Config, GenericAsset, GenericPrice, GenericQty, Module, MultiplicativeIndex, Reason,
};

macro_rules! require {
    ($expr:expr, $reason:expr) => {
        if !$expr {
            return core::result::Result::Err($reason);
        }
    };
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

/// XXX doc
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price<const S: Symbol>(pub GenericPrice);

/// XXX doc
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quantity<const S: Symbol>(pub GenericQty);

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

impl<const S: Symbol> Mul<Quantity<S>> for Price<S> {
    type Output = Quantity<{ Symbol::USD }>;

    fn mul(self, rhs: Quantity<S>) -> Self::Output {
        Quantity(self.0 * rhs.0)
    }
}

impl<const S: Symbol> Mul<Price<S>> for Quantity<S> {
    type Output = Quantity<{ Symbol::USD }>;

    fn mul(self, rhs: Price<S>) -> Self::Output {
        Quantity(self.0 * rhs.0)
    }
}

impl<const S: Symbol> Mul<MultiplicativeIndex> for Quantity<S> {
    type Output = Quantity<S>;

    fn mul(self, rhs: u128) -> Self::Output {
        Quantity(self.0 * rhs) // XXX index will probably be wrapped
    }
}

impl<const S: Symbol> Div<Price<S>> for Quantity<{ Symbol::USD }> {
    type Output = Quantity<{ S }>;

    fn div(self, rhs: Price<S>) -> Self::Output {
        Quantity(self.0 / rhs.0)
    }
}

impl<const S: Symbol> Div<Quantity<S>> for Quantity<{ Symbol::USD }> {
    type Output = Price<S>;

    fn div(self, rhs: Quantity<S>) -> Self::Output {
        Price(self.0 / rhs.0)
    }
}

pub fn price<T: Config, const S: Symbol>() -> Price<S> {
    match S {
        Symbol::CASH => Price(1), // XXX $1, Price::from_nominal(1)?
        _ => Price(<Module<T>>::prices(S)),
    }
}

/// Protocol Interface

pub fn apply_eth_event_internal(event: eth::Event) -> Result<eth::Event, Reason> {
    Ok(event) // XXX
}

pub fn extract_internal<T: Config, C: Chain, const S: Symbol>(
    asset: Asset<C>,
    holder: Account<Compound>,
    recipient: Account<C>,
    principal: Quantity<S>,
) -> Result<(), Reason> {
    // Require Recipient.Chain=Asset.Chain XXX proven by compiler
    let supply_index = <Module<T>>::supply_index(Into::<GenericAsset>::into(asset));
    let amount = principal * supply_index;
    let value = amount * price::<T, S>();
    require!(value >= MIN_TX_VALUE, Reason::MinTxValueNotMet);

    // Read Require HasLiquidityToReduceCollateralAsset(Holder, Asset, Amount)
    // ReadsCashBorrowPrincipalBorrower, CashCostIndexPair, CashYield, CashSpread, Price*, SupplyPrincipal*, Borrower, StabilityFactor*
    // Read TotalSupplyNew=TotalSupplyPrincipalAsset-Principal
    // Underflow: Not enough total funds to extract ${Amount}
    // Read HolderSupplyNew=SupplyPrincipalAsset, Holder-Principal
    // Underflow: ${Holder} does not have enough funds to extract ${Amount}
    // Set TotalSupplyPrincipalAsset=TotalSupplyNew
    // Set SupplyPrincipalAsset, Holder=HolderSupplyNew
    // Add ExtractionNotice(Asset, Recipient, Amount) to NoticeQueueRecipient.Chain
    Ok(()) // XXX
}

// XXX should we expect amounts are already converted to our bigint type here?
//  probably not, probably inputs should always be fixed width?
//   actually now I think we can always guarantee to parse ascii numbers in lisp requests into bigints
pub fn extract_cash_principal_internal<T: Config, C: Chain>(
    holder: Account<Compound>,
    recipient: Account<C>,
    principal: Quantity<{ Symbol::CASH }>,
) -> Result<(), Reason> {
    let yield_index = <Module<T>>::cash_yield_index();
    let amount = principal * yield_index;
    let value = amount * price::<T, { Symbol::CASH }>();
    require!(value >= MIN_TX_VALUE, Reason::MinTxValueNotMet);

    // Note: we do not check health here, since CASH cannot be borrowed against yet.
    // let chain_cash_hold_principal_new = <Module<T>>::chain_cash_hold_principal(recipient.chain) + amount_principal;
    // let holder_cash_hold_principal_new = <Module<T>>::cash_hold_principal(holder) - amount_principal;
    // XXX Underflow: ${Account} does not have enough CASH to extract ${Amount};
    // <Module<T>>::ChainCashHoldPrincipal::insert(recipient, chain_cash_hold_principal_new);
    // <Module<T>>::C
    //     Set TotalCashHoldPrincipalRecipient.Chain=ChainCashHoldPrincipalNew;
    //     Set CashHoldPrincipalHolder=HolderCashHoldPrincipalNew;
    //     Add CashExtractionNotice(Recipient, Amount, YieldIndex) to NoticeQueueRecipient.Chain;
    Ok(()) // XXX
}

#[cfg(test)]
mod tests {
    // XXX
}
