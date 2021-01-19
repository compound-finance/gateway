// Note: The substrate build requires these be re-exported.
pub use our_std::{fmt, result, result::Result};

/// Setup
use codec::{Decode, Encode};
use our_std::RuntimeDebug;

use crate::{
    chains::{eth, Chain, ChainId, Ethereum},
    notices::Notice, // XXX move here, encoding to chains
    params::MIN_TX_VALUE,
    types::ChainSignature,
    CashBalance,
    Config,
    Module,
    RawEvent::GoldieLocks,
    Store,
};
use sp_runtime::print;

macro_rules! require {
    ($expr:expr, $reason:expr) => {
        if !$expr {
            return core::result::Result::Err($reason);
        }
    };
}

macro_rules! require_min_tx_value {
    ($value:expr) => {
        require!($value >= MIN_TX_VALUE, Reason::MinTxValueNotMet);
    };
}

/// Type for reporting failures for reasons outside of our control.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum Reason {
    None,
    NotImplemented,
    MinTxValueNotMet,
    InvalidSymbol,
}

// Protocol interface //

pub fn apply_eth_event_internal<T: Config>(event: eth::Event) -> Result<(), Reason> {
    match event.data {
        eth::EventData::Lock {
            asset,
            holder,
            amount,
        } => {
            //  When Lock(Asset:address, Holder:address, Amount:uint256):
            //   Build AccountIdent=("eth", account)
            //   Build AssetIdent=("eth", asset)
            //   Call lockInternal(AssetIdent, AccountIdent, Amount)
            print("applying lock event...");
            lock_internal::<T, Ethereum>(
                Asset(asset),
                Account(holder),
                Quantity(symbol::<T, Ethereum>(Asset(asset)), amount.into()),
            )
        }
        _ => {
            //  When Gov(title:string, extrinsics:bytes[]):
            //   Decode a SCALE-encoded set of extrinsics from the event
            //   For each extrinsic, dispatch the given extrinsic as Root
            //  When LockCash(Account:address, Amount: uint256, CashYieldIndex: uint256):
            //   Build AccountIdent=("eth", account)
            //   Call lockCashInternal(AccountIdent, Amount)
            Err(Reason::NotImplemented)
        }
    }
}

pub fn lock_internal<T: Config, C: Chain>(
    asset: Asset<C>,
    holder: Account<C>,
    amount: Quantity,
) -> Result<(), Reason> {
    print("lock internal...");

    Module::<T>::deposit_event(GoldieLocks(asset.into(), holder.into(), amount.into()));

    // XXX
    // Read Require AmountPriceAssetParamsMinTxValue
    // Read Principal =AmountSupplyIndexAsset
    // Read TotalSupplyNew=TotalSupplyPrincipalAsset+Principal
    // Read HolderSupplyNew=SupplyPrincipalAsset, Holder+Principal
    // Set TotalSupplyPrincipalAsset=TotalSupplyNew
    // Set SupplyPrincipalAsset, Holder=HolderSupplyNew
    Ok(())
}

pub fn lock_cash_internal<T: Config, C: Chain>(
    holder: Account<C>,
    amount: Quantity, // XXX CashQuantity?
) -> Result<(), Reason> {
    // XXX
    // Read Require AmountPriceCASHParamsMinTxValue
    // Read Principal =AmountCashYieldIndex
    // Read ChainCashHoldPrincipalNew=TotalCashHoldPrincipalHolder.Chain-Principal
    // Underflow: ${Sender.Chain} does not have enough total CASH to extract ${Amount}
    // Read HolderCashHoldPrincipalNew=CashHoldPrincipalHolder+Principal
    // Set TotalCashHoldPrincipalHolder.Chain=ChainCashHoldPrincipalNew
    // Set CashHoldPrincipalHolder=HolderCashHoldPrincipalNew
    Ok(())
}

pub fn extract_principal_internal<T: Config, C: Chain>(
    asset: Asset<C>,
    holder: Account<C>,
    recipient: Account<C>,
    principal: Quantity,
) -> Result<(), Reason> {
    // Require Recipient.Chain=Asset.Chain XXX proven by compiler
    let supply_index = <Module<T>>::supply_index(Into::<GenericAsset>::into(asset));
    let amount = principal * supply_index;
    require_min_tx_value!(amount * price::<T>(principal.symbol()));

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
    holder: Account<C>,
    recipient: Account<C>,
    principal: Quantity, // XXX CashQuantity?
) -> Result<(), Reason> {
    let yield_index = <Module<T>>::cash_yield_index();
    let amount = principal * yield_index;
    require_min_tx_value!(amount * price::<T>(CASH));

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

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum RecoveryError {
    RecoveryError,
}

pub fn get_signer(message: &[u8], sig: ChainSignature) -> Result<ChainAccount, RecoveryError> {
    match sig {
        ChainSignature::Eth(eth_sig) => {
            let account =
                eth::recover(message, eth_sig).map_err(|_| RecoveryError::RecoveryError)?;
            Ok(ChainAccount::Eth(account))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
