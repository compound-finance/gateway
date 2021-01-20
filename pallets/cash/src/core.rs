// Note: The substrate build requires these be re-exported.
pub use our_std::{fmt, result, result::Result};

/// Setup
use codec::{Decode, Encode};
use our_std::{
    convert::TryInto,
    ops::{Div, Mul},
    RuntimeDebug,
};

use crate::{
    chains::{eth, Chain, ChainId, Ethereum},
    notices::Notice, // XXX move here, encoding to chains
    params::MIN_TX_VALUE,
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

// Type aliases //

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
pub type SigData = Vec<u8>;

/// Type for a generic signature, potentially for any chain.
pub type GenericSig = (ChainId, SigData);

/// Type for a bunch of generic signatures.
pub type GenericSigs = Vec<SigData>;

/// Type for representing a price, potentially for any symbol.
pub type GenericPrice = Uint;

/// Type for representing a quantity, potentially of any symbol.
pub type GenericQty = Uint;

/// Type for representing a quantity, potentially of any symbol.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum GenericMaxQty {
    Max,
    Qty(GenericQty),
}

impl From<u128> for GenericMaxQty {
    fn from(amt: u128) -> Self {
        GenericMaxQty::Qty(amt)
    }
}

/// Type for chain signatures
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainSignature {
    Eth(<Ethereum as Chain>::Signature),
}

/// Type for chain accounts
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainAccount {
    Eth(<Ethereum as Chain>::Address),
}

/// Type for a generic set, for validators/reporters in the genesis config.
pub type GenericSet = Vec<String>;

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

// Type definitions //

/// Type for reporting failures for reasons outside of our control.
#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum Reason {
    None,
    NotImplemented,
    MinTxValueNotMet,
    InvalidSymbol,
}

/// Type for the abstract symbol of an asset, not tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug)]
pub struct Symbol(pub [char; 12], pub u8);

// Define symbols used directly by the chain itself
pub const NIL: char = 0 as char;
pub const CASH: Symbol = Symbol(
    ['C', 'A', 'S', 'H', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
    6,
);
pub const USD: Symbol = Symbol(
    ['U', 'S', 'D', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
    6,
);

impl Symbol {
    pub const fn ticker(&self) -> &[char] {
        &self.0
    }

    pub const fn decimals(&self) -> u8 {
        self.1
    }
}

impl Encode for Symbol {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        let mut bytes: Vec<u8> = self.0.to_vec().iter().map(|&c| c as u8).collect();
        bytes.push(self.1);
        bytes.using_encoded(f)
    }
}

impl codec::EncodeLike for Symbol {}

impl Decode for Symbol {
    fn decode<I: codec::Input>(encoded: &mut I) -> Result<Self, codec::Error> {
        let mut bytes: Vec<u8> = Decode::decode(encoded)?;
        let decimals = bytes.pop().unwrap();
        let chars: Vec<char> = bytes.iter().map(|&b| b as char).collect();
        let ticker: [char; 12] = chars.try_into().expect("wrong number of chars");
        Ok(Symbol(ticker, decimals))
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
pub enum NoticeStatus {
    Missing,
    Pending {
        signers: crate::ValidatorSet,
        signatures: GenericSigs,
        notice: Notice,
    },
    Done,
}

/// Type for representing an account bound to a specific chain.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Account<C: Chain>(pub C::Address);

/// Type for representing an asset bound to a specific chain.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Asset<C: Chain>(pub C::Address);

impl<C: Chain> From<Asset<C>> for GenericAsset {
    fn from(asset: Asset<C>) -> Self {
        (C::ID, asset.0.into())
    }
}

impl<C: Chain> From<Account<C>> for GenericAccount {
    fn from(account: Account<C>) -> Self {
        (C::ID, account.0.into())
    }
}

impl From<Quantity> for GenericQty {
    fn from(quantity: Quantity) -> Self {
        quantity.amount().into()
    }
}

/// Type for representing a price (in USD), bound to its symbol.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct Price(pub Symbol, pub GenericPrice);

/// Type for representing a quantity of an asset, bound to its symbol.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct Quantity(pub Symbol, pub GenericQty);

/// Type for representing a multiplicative index on Compound Chain.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct MulIndex(pub Uint);

impl Price {
    pub const DECIMALS: u8 = USD.decimals(); // Note: must be >= USD.decimals()

    pub const fn symbol(&self) -> Symbol {
        self.0
    }

    pub const fn amount(&self) -> GenericPrice {
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

    pub const fn amount(&self) -> GenericQty {
        self.1
    }

    pub const fn from_nominal(symbol: Symbol, nominal: f64) -> Self {
        Quantity(symbol, (nominal * pow10(symbol.decimals())) as Uint)
    }

    pub const fn to_nominal(&self) -> f64 {
        (self.amount() as f64) / pow10(self.symbol().decimals())
    }
}

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
                / (pow10(Price::DECIMALS + rhs.symbol().decimals() - USD.decimals()) as GenericQty),
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
                / (pow10(Price::DECIMALS + self.symbol().decimals() - USD.decimals())
                    as GenericQty),
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
                * (pow10(Price::DECIMALS + rhs.symbol().decimals() - USD.decimals())
                    as GenericPrice)
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
                * (pow10(Price::DECIMALS + rhs.symbol().decimals() - USD.decimals()) as GenericQty)
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

pub fn symbol<T: Config, C: Chain>(asset: Asset<C>) -> Symbol {
    // XXX lookup in storage
    Symbol(
        ['E', 'T', 'H', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
        18,
    )
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

pub fn get_quantity(max: GenericMaxQty, when_max: &dyn Fn() -> GenericQty) -> GenericQty {
    match max {
        GenericMaxQty::Max => when_max(),
        GenericMaxQty::Qty(qty) => qty,
    }
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
