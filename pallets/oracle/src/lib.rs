#![feature(array_methods)]

use crate::{
    error::OracleError,
    ticker::{Ticker, CASH_TICKER, USD_TICKER},
    types::{AssetPrice, Price, ReporterSet, Timestamp},
};
use frame_support::{
    decl_event, decl_module, decl_storage, dispatch,
    traits::UnfilteredDispatchable,
    weights::{DispatchClass, GetDispatchInfo, Pays},
    Parameter,
};
use frame_system::{ensure_none, offchain::CreateSignedTransaction};
use our_std::log;
use sp_runtime::transaction_validity::{
    InvalidTransaction, TransactionSource, TransactionValidity,
};

pub mod error;
pub mod inherent;
pub mod oracle;
pub mod serdes;
pub mod ticker;
pub mod types;
pub mod validate_trx;

#[cfg(test)]
mod tests;

/// Number of blocks between HTTP requests from offchain workers to open oracle price feed.
pub const ORACLE_POLL_INTERVAL_BLOCKS: u32 = 10;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;

    /// The overarching dispatch call type.
    type Call: From<Call<Self>>
        + Parameter
        + UnfilteredDispatchable<Origin = Self::Origin>
        + GetDispatchInfo;
}

decl_storage! {
    trait Store for Module<T: Config> as Cash {
        /// Mapping of latest prices for each price ticker.
        pub Prices get(fn price): map hasher(blake2_128_concat) Ticker => Option<AssetPrice>;

        /// Mapping of assets to the last time their price was updated.
        pub PriceTimes get(fn price_time): map hasher(blake2_128_concat) Ticker => Option<Timestamp>;

        /// Ethereum addresses of open oracle price reporters.
        pub PriceReporters get(fn reporters): ReporterSet; // XXX if > 1, how are we combining?
    }
    add_extra_genesis {
        config(reporters): ReporterSet;
        build(|config| {
            Module::<T>::initialize_reporters(config.reporters.clone());
        })
    }
}

/* ::EVENTS:: */

decl_event!(
    pub enum Event {
        /// Failed to process a given extrinsic. [reason]
        Failure(OracleError),
    }
);

/* ::ERRORS:: */

fn check_failure<T: Config>(res: Result<(), OracleError>) -> Result<(), OracleError> {
    if let Err(err) = res {
        <Module<T>>::deposit_event(Event::Failure(err));
        log!("Oracle Failure {:#?}", err);
    }
    res
}

/* ::MODULE:: */
/* ::EXTRINSICS:: */

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

        /// Set the price using the open price feed. [User] [Free]
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn post_price(origin, payload: Vec<u8>, signature: Vec<u8>) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Ok(check_failure::<T>(oracle::post_price::<T>(payload, signature))?)
        }

        /// Set several prices using the open price feed. [User] [Free]
        #[weight = (1, DispatchClass::Operational, Pays::No)] // XXX
        pub fn post_prices(origin, pairs: Vec<(Vec<u8>, Vec<u8>)>) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Ok(pairs.into_iter().fold(Ok(()) as Result<(), OracleError>, |res, (payload, signature)| {
                match res {
                    Err(err) => Err(err),
                    Ok(_) => check_failure::<T>(oracle::post_price::<T>(payload, signature)),
                }
            })?)
        }

        /// Offchain Worker entry point.
        fn offchain_worker(block_number: T::BlockNumber) {
            if let Err(e) = oracle::process_prices::<T>(block_number) {
                log!("offchain_worker error during open price feed processing: {:?}", e);
            }
        }
    }
}

/// Return the USD price associated with the given units.
pub fn get_price_by_ticker<T: Config>(ticker: Ticker) -> Option<Price> {
    match ticker {
        t if t == USD_TICKER => Some(Price::from_nominal(USD_TICKER, "1.0")),
        t if t == CASH_TICKER => Some(Price::from_nominal(CASH_TICKER, "1.0")),
        _ => Prices::get(ticker).map(|price| Price::new(ticker, price)),
    }
}

/// Reading error messages inside `decl_module!` can be difficult, so we move them here.
impl<T: Config> Module<T> {
    /// Set the initial set of open price feed price reporters from the genesis config
    pub fn initialize_reporters(reporters: ReporterSet) {
        assert!(
            !reporters.is_empty(),
            "Open price feed price reporters must be set in the genesis config"
        );
        PriceReporters::put(reporters);
    }

    // ** API / View Functions ** //

    /// Get the price for the given asset.
    pub fn get_price(ticker: Ticker) -> Result<AssetPrice, OracleError> {
        Ok(get_price_by_ticker::<T>(ticker)
            .ok_or(OracleError::NoPrice)?
            .value)
    }
}

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    /// Validate unsigned call to this module.
    ///
    /// By default unsigned transactions are disallowed, but implementing the validator
    /// here we make sure that some particular calls (the ones produced by offchain worker)
    /// are being whitelisted and marked as valid.
    fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
        validate_trx::check_validation_failure(
            call,
            validate_trx::validate_unsigned::<T>(source, call),
        )
        .unwrap_or(InvalidTransaction::Call.into())
    }
}
