// Note: The substrate build requires these be re-exported.
use codec::{Decode, Encode};
pub use our_std::{fmt, result, result::Result};
use our_std::{vec::Vec, RuntimeDebug};

use crate::{
    amount::CashAmount,
    chains::{eth, ChainId},
    params, // XXX
    Config,
    Module,
    Nonce,
    Reason,
};

// XXX these types can still use another pass
//  further clarification around when to use this vs chain-specific type
/// Type for a generic address, potentially on any chain.
pub type Address = Vec<u8>;

/// Type for a generic account identifier, tied to one of the possible chains.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct AccountId {
    pub chain: ChainId,
    pub address: Address,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Default)]
pub struct AssetId {
    pub chain: ChainId,
    pub address: Address,
}

pub enum AssetOrCash {
    Cash,
    Asset(AssetId),
}

pub type Price = u128; // XXX

// XXX
pub fn require(predicate: bool, reason: &'static str) {
    if !predicate {
        panic!(reason); // XXX
    }
}

// XXX
pub fn price(asset: AssetOrCash) -> Price {
    match asset {
        AssetOrCash::Cash => 1, // XXX $1
        AssetOrCash::Asset(asset_id) => panic!("xxx no prices yet"),
    }
}

pub fn apply_eth_event_internal(event: eth::Event) -> Result<eth::Event, Reason> {
    Ok(event) // XXX
}

// XXX should we expect amounts are already converted to our bigint type here?
//  probably not, probably inputs should always be fixed width?
pub fn extract_cash_principal_internal<T: Config>(
    holder: AccountId,
    recipient: AccountId,
    amount_principal: CashAmount,
    nonce: Nonce,
) -> Result<(), Reason> {
    // XXX
    require(
        amount_principal * price(AssetOrCash::Cash) >= params::MIN_TX_VALUE,
        "min tx value not met",
    ); // XXX
    require(
        nonce == <Module<T>>::nonces(holder) + 1,
        "supplied nonce does not match",
    ); // XXX
       // Note: we do not check health here, since CASH cannot be borrowed against yet.
       //     Read YieldIndex=CashYieldIndex;
       //     Read AmountPrincipal =AmountYieldIndex;
       //     Read ChainCashHoldPrincipalNew=TotalCashHoldPrincipalRecipient.Chain+AmountPrincipal;
       //     Read HolderCashHoldPrincipalNew=CashHoldPrincipalHolder-AmountPrincipal;
       //       Underflow: ${Account} does not have enough CASH to extract ${Amount};
       //     Set NoncesHolder=Nonce;
       //     Set TotalCashHoldPrincipalRecipient.Chain=ChainCashHoldPrincipalNew;
       //     Set CashHoldPrincipalHolder=HolderCashHoldPrincipalNew;
       //     Add CashExtractionNotice(Recipient, Amount, YieldIndex) to NoticeQueueRecipient.Chain;
    Ok(()) // XXX
}

#[cfg(test)]
mod tests {
    // XXX
}
