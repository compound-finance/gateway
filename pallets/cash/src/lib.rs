#![allow(incomplete_features)]
#![feature(array_methods)]
#![feature(associated_type_defaults)]
#![feature(const_fn_floating_point_arithmetic)]

#[macro_use]
extern crate alloc;
extern crate ethereum_client;
extern crate trx_request;

use codec::{alloc::string::String, Decode};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get,
};
use frame_system::{
    ensure_none,
    offchain::{CreateSignedTransaction, SubmitTransaction},
};
use our_std::{
    convert::{TryFrom, TryInto},
    str,
    vec::Vec,
};
use sp_runtime::{
    offchain::{
        storage::StorageValueRef,
        storage_lock::{StorageLock, Time},
    },
    transaction_validity::{
        InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
    },
    SaturatedConversion,
};

use crate::chains::{Chain, ChainId, Ethereum};
use crate::core::get_signer;
use crate::core::Reason;
use crate::notices::{CashExtractionNotice, EncodeNotice, Notice, NoticeId};
use crate::symbol::Symbol;
use crate::types::{
    get_max_value, AssetAmount, AssetPrice, ChainAccount, ChainAsset, ChainSignature,
    ChainSignatureList, ConfigSetString, EncodedNotice, EventStatus, MaxableAssetAmount, MulIndex,
    Nonce, NoticeStatus, ReporterSet, SignedPayload, Timestamp, ValidatorSet, ValidatorSig, APR,
};

use sp_runtime::print;

mod chains;
mod core;
mod events;
mod notices;
mod oracle;
mod params;
mod sig;
mod symbol;
mod trx_req;
mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// OCW storage constants
pub const OCW_STORAGE_LOCK_ETHEREUM_EVENTS: &[u8; 34] = b"cash::storage_lock_ethereum_events";
pub const OCW_LATEST_CACHED_ETHEREUM_BLOCK: &[u8; 34] = b"cash::latest_cached_ethereum_block";
pub const OCW_LATEST_PRICE_FEED_POLL_BLOCK_NUMBER: &[u8; 41] =
    b"cash::latest_price_feed_poll_block_number";
pub const OCW_STORAGE_LOCK_OPEN_PRICE_FEED: &[u8; 34] = b"cash::storage_lock_open_price_feed";

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// The overarching dispatch call type.
    type Call: From<Call<Self>>;
}

decl_storage! {
    trait Store for Module<T: Config> as Cash {
        /// The timestamp of the previous block (or defaults to timestamp of the genesis block).
        LastBlockTimestamp get(fn last_block_timestamp) config(): Timestamp;

        // XXX we also need mapping of public (identity, babe) key to other keys
        /// The current set of allowed validators, and their associated keys.
        Validators get(fn validators): ValidatorSet; // XXX

        /// The upcoming set of allowed validators, and their associated keys (or none).
        NextValidators get(fn next_validators): Option<ValidatorSet>; // XXX

        /// An index to track interest owed by CASH borrowers.
        CashCostIndex get(fn cash_cost_index): MulIndex;

        /// An index to track interest earned by CASH holders.
        CashYieldIndex get(fn cash_yield_index): MulIndex;

        /// The upcoming base rate change for CASH and when, if any.
        CashYieldNext get(fn cash_yield_next): Option<(APR, Timestamp)>;

        /// The current APR on CASH held, and the base rate paid by borrowers.
        CashYield get(fn cash_yield): APR;

        /// The current APR surcharge on CASH borrowed, added to the CashYield to determine the CashCost.
        CashSpread get(fn cash_spread): APR;

        /// An index to track interest owed by asset borrowers.
        BorrowIndex get(fn borrow_index): map hasher(blake2_128_concat) ChainAsset => MulIndex; // XXX will change to add

        /// An index to track interest earned by asset suppliers.
        SupplyIndex get(fn supply_index): map hasher(blake2_128_concat) ChainAsset => MulIndex; // XXX will change to add

        // XXX doc
        ChainCashHoldPrincipal get(fn chain_cash_hold_principal): map hasher(blake2_128_concat) ChainId => AssetAmount;
        // TotalCashHoldPrincipal;
        // TotalCashBorrowPrincipal;
        // TotalSupplyPrincipal[asset];
        // TotalBorrowPrincipal[asset];
        // CashHoldPrincipal[account];
        // CashBorrowPrincipal[account];
        // SupplyPrincipal[asset][account];
        // BorrowPrincipal[asset][account];

        /// The last used nonce for each account, initialized at zero.
        Nonces get(fn nonces): map hasher(blake2_128_concat) ChainAccount => Nonce;

        // XXX delete me (part of magic extract)
        pub CashBalance get(fn cash_balance): map hasher(blake2_128_concat) ChainAccount => Option<AssetAmount>;

        /// Mapping of (status of) events witnessed on Ethereum, by event id.
        EthEventQueue get(fn eth_event_queue): map hasher(blake2_128_concat) chains::eth::EventId => Option<EventStatus<Ethereum>>;

        /// Mapping of (status of) notices to be signed for Ethereum, by notice id.
        EthNoticeQueue get(fn eth_notice_queue): map hasher(blake2_128_concat) NoticeId => Option<NoticeStatus>;

        /// Eth addresses of price reporters for open oracle
        Reporters get(fn reporters): ReporterSet;

        // XXX
        // AssetInfo[asset];
        // LiquidationIncentive;

        /// Mapping of strings to symbols.
        Symbols get(fn symbols): map hasher(blake2_128_concat) String => Option<Symbol>;

        /// Mapping of latest prices for each asset symbol.
        Prices get(fn prices): map hasher(blake2_128_concat) Symbol => AssetPrice;

        /// Mapping of assets to the last time their price was updated.
        PriceTimes get(fn price_times): map hasher(blake2_128_concat) Symbol => Timestamp;

        /// Mapping of assets to symbols.
        PriceKeyMapping get(fn price_key_mapping): map hasher(blake2_128_concat) ChainAsset => Option<Symbol>;

        // PriceReporter;
        // PriceKeyMapping;
    }
    add_extra_genesis {
        config(validators): ConfigSetString;
        config(reporters): ConfigSetString;
        config(symbols): ConfigSetString; // XXX initialize symbol from string: (ticker, decimals)
        config(price_key_mapping): ConfigSetString;
        build(|config| {
            Module::<T>::initialize_validators(config.validators.clone());
            Module::<T>::initialize_reporters(config.reporters.clone());
            Module::<T>::initialize_symbols(config.symbols.clone());
            Module::<T>::initialize_price_key_mapping(config.price_key_mapping.clone());
        })
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId, //XXX seems to require a where clause with reference to T to compile
    {
        XXXPhantomFakeEvent(AccountId), // XXX

        /// XXX -- For testing
        GoldieLocks(ChainAsset, ChainAccount, AssetAmount),

        /// XXX
        MagicExtract(AssetAmount, ChainAccount, Notice),

        /// An Ethereum event was successfully processed. [payload]
        ProcessedEthEvent(SignedPayload),

        /// An Ethereum event failed during processing. [payload, reason]
        FailedProcessingEthEvent(SignedPayload, Reason),

        /// Signed notice. [chain_id, notice_id, message, signatures]
        SignedNotice(ChainId, NoticeId, EncodedNotice, ChainSignatureList),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        // XXX delete with magic extract
        /// Error returned when fetching starport info
        HttpFetchingError,

        /// Error when processing `Lock` event while sending `process_eth_event` extrinsic
        ProcessLockEventError,

        /// Error sending `process_eth_event` extrinsic
        OffchainUnsignedLockTxError,

        /// Error decoding payload
        SignedPayloadError,

        /// Public key doesn't belong to known validator
        UnknownValidator,

        /// Validator has already signed and submitted this payload
        AlreadySigned,

        /// Fetched Ethereum event type is not known
        UnknownEthEventType, // XXX needed per-chain?

        /// Error decoding Ethereum event
        DecodeEthereumEventError,

        /// An error related to the oracle
        OpenOracleError,

        /// Open oracle cannot parse the signature
        OpenOracleErrorInvalidSignature,

        /// Open oracle cannot parse the signature
        OpenOracleErrorInvalidReporter,

        /// Open oracle cannot parse the message
        OpenOracleErrorInvalidMessage,

        /// Open oracle cannot update price due to stale price
        OpenOracleErrorStalePrice,

        /// Open oracle cannot fetch price due to an http error
        OpenOracleHttpFetchError,

        /// Open oracle cannot deserialize the messages and/or signatures from eth encoded hex to binary
        OpenOracleApiResponseHexError,

        /// Open oracle offchain worker made an attempt to update the price but failed in the extrinsic
        OpenOraclePostPriceExtrinsicError,

        /// An unsupported symbol was provided to the oracle.
        OpenOracleErrorInvalidSymbol,

        /// An error related to the chain_spec file contents
        GenesisConfigError,

        /// Trx request parsing error
        TrxRequestParseError,

        /// Signature recovery errror
        InvalidSignature,

        /// Sender issue
        InvalidChain,

        /// Failure in internal logic
        Failed,

        /// Notice issue
        InvalidNoticeParams,
    }
}

macro_rules! r#cash_err {
    ($expr:expr, $new_err:expr) => {
        match $expr {
            core::result::Result::Ok(val) => Ok(val),
            core::result::Result::Err(err) => {
                debug::native::error!("Error {:#?}", err);
                Err($new_err)
            }
        }
    };
}

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

        /// Set the price using the open price feed.
        #[weight = 0]
        pub fn post_price(origin, payload: Vec<u8>, signature: Vec<u8>) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            Self::post_price_internal(payload, signature)?;
            Ok(())
        }

        // TODO
        #[weight = 1]
        pub fn exec_trx_request(origin, request: Vec<u8>, signature: ChainSignature) -> dispatch::DispatchResult {
            ensure_none(origin)?;

            // // TODO: Add more error information here
            print("submit_trx_request");
            let request_str: &str = str::from_utf8(&request[..]).map_err(|_| <Error<T>>::TrxRequestParseError)?;
            print(request_str);
            // // TODO: Add more error information here
            let sender = cash_err!(get_signer(&chains::eth::prepend_eth_signing_msg(&request), signature), Error::<T>::InvalidSignature)?;
            let trx_request = trx_request::parse_request(request_str).map_err(|_| <Error<T>>::TrxRequestParseError)?;

            match trx_request {
                trx_request::TrxRequest::MagicExtract(amount, account) =>{
                    magic_extract_internal::<T>(sender, account.into(), amount.into()).map_err(|r| Error::<T>::Failed)?; // TODO: How to handle wrapped errors?
                    Ok(())
                }
            }
        }

        #[weight = 1] // XXX how are we doing weights?
        pub fn process_eth_event(origin, payload: SignedPayload, signature: ValidatorSig) -> dispatch::DispatchResult { // XXX sig
            print(format!("process_eth_event(origin,payload,sig): {} {}", hex::encode(&payload), hex::encode(&signature)).as_str());
            ensure_none(origin)?;

            // XXX do we want to store/check hash to allow replaying?
            // TODO: use more generic function?
            let signer = cash_err!(
                chains::eth::recover(&payload[..], signature),
                Error::<T>::InvalidSignature)?;

            print(format!("signer: {}", hex::encode(&signer)).as_str());
            // XXX
            // XXX require(signer == known validator); // XXX
            // Check that signer is a known validator, otherwise throw an error
            let validators = <Validators>::get();
            if !validators.contains(&signer) {
                debug::native::error!("Signer of a payload is not a known validator {:?}, validators are {:?}", signer, validators);
                return Err(Error::<T>::UnknownValidator)?
            }

            let event = chains::eth::Event::decode(&mut payload.as_slice()).map_err(|_| <Error<T>>::DecodeEthereumEventError)?; // XXX
            let status = <EthEventQueue>::get(event.id).unwrap_or(EventStatus::<Ethereum>::Pending { signers: vec![] }); // XXX
            match status {
                EventStatus::<Ethereum>::Pending { signers } => {
                    // XXX sets?
                    if signers.contains(&signer) {
                        debug::native::error!("Validator has already signed this payload {:?}", signer);
                        return Err(Error::<T>::AlreadySigned)?
                    }

                    // Add new validator to the signers
                    let mut signers_new = signers.clone();
                    signers_new.push(signer.clone()); // XXX unique add to set?

                    // XXX
                    // let signers_new = {signer | signers};
                    // if len(signers_new & Validators) > 2/3 * len(Validators) {
                    if signers_new.len() > validators.len() * 2 / 3 {
                        match core::apply_eth_event_internal::<T>(event) {
                            Ok(()) => {
                                EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Done);
                                Self::deposit_event(RawEvent::ProcessedEthEvent(payload));
                                Ok(())
                            }

                            Err(reason) => {
                                EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Failed { hash: Ethereum::hash_bytes(payload.as_slice()), reason: reason });
                                Self::deposit_event(RawEvent::FailedProcessingEthEvent(payload, reason));
                                Ok(())
                            }
                        }
                    } else {
                        EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Pending { signers: signers_new });
                        Ok(())
                    }
                }
                // XXX potential retry logic to allow retrying Failures
                EventStatus::<Ethereum>::Failed { hash, .. } => {
                    // XXX require(compute_hash(payload) == hash, "event data differs from failure");
                    match core::apply_eth_event_internal::<T>(event) {
                        Ok(_) => {
                            EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Done);
                            Self::deposit_event(RawEvent::ProcessedEthEvent(payload));
                            Ok(())
                        }

                        Err(new_reason) => {
                            EthEventQueue::insert(event.id, EventStatus::<Ethereum>::Failed { hash, reason: new_reason});
                            Self::deposit_event(RawEvent::FailedProcessingEthEvent(payload, new_reason));
                            Ok(())
                        }
                    }
                }

                EventStatus::<Ethereum>::Done => {
                    // TODO: Eventually we should be deleting here (based on monotonic ids and signing order)
                    Ok(())
                }
            }
        }

        #[weight = 0] // XXX
        pub fn publish_eth_signature(origin, notice_id: NoticeId, signature: ValidatorSig) -> dispatch::DispatchResult {
            //let signer = recover(payload); // XXX
            //require(signer == known validator); // XXX

            let status = <EthNoticeQueue>::get(notice_id).unwrap_or(NoticeStatus::Missing);
            match status {
                NoticeStatus::Missing => {
                    Ok(())
                }

                NoticeStatus::Pending { signers, signatures, notice } => {
                    // XXX sets?
                    // let signers_new = {signer | signers};
                    // let signatures_new = {signature | signatures };
                    // if len(signers_new & Validators) > 2/3 * len(Validators) {
                           EthNoticeQueue::insert(notice_id, NoticeStatus::Done);
                           Self::deposit_event(RawEvent::SignedNotice(ChainId::Eth, notice_id, notice.encode_ethereum_notice(), signatures)); // XXX generic
                           Ok(())
                    // } else {
                    //     EthNoticeQueue::insert(event.id, NoticeStatus::Pending { signers: signers_new, signatures, notice});
                    //     Ok(())
                    // }
                }

                NoticeStatus::Done => {
                    // TODO: Eventually we should be deleting here (based on monotonic ids and signing order)
                    Ok(())
                }
            }
        }

        /// Offchain Worker entry point.
        fn offchain_worker(block_number: T::BlockNumber) {
            debug::native::info!("Hello World from offchain workers from block {} !", block_number);

            let result = Self::fetch_events_with_lock();
            if let Err(e) = result {
                debug::native::error!("offchain_worker error during fetch events: {:?}", e);
            }
            Self::process_eth_notices(block_number);
            // todo: do this less often perhaps once a minute?
            // a really simple idea to do it once a minute on average is to just use probability
            // and only update the feed every now and then. It is a function of how many validators
            // are running.
            let result = Self::process_open_price_feed(block_number);
            if let Err(e) = result {
                debug::native::error!("offchain_worker error during open price feed processing: {:?}", e);
            }
        }
    }
}

/// Reading error messages inside `decl_module!` can be difficult, so we move them here.
impl<T: Config> Module<T> {
    /// Body of the post_price extrinsic
    /// * checks signature of message
    /// * parses message
    /// * updates storage
    ///
    /// Gets us out of the decl_module! macro and helps with static code analysis to have the
    /// body of this function here.
    fn post_price_internal(payload: Vec<u8>, signature: Vec<u8>) -> Result<(), Error<T>> {
        // check signature
        let hashed = compound_crypto::keccak(&payload);
        let recovered = cash_err!(
            compound_crypto::eth_recover(&hashed, &signature),
            <Error<T>>::OpenOracleErrorInvalidSignature
        )?;
        let reporters = Reporters::get();
        if !reporters
            .iter()
            .any(|e| e.as_slice() == recovered.as_slice())
        {
            return Err(<Error<T>>::OpenOracleErrorInvalidReporter);
        }

        // parse message and check it
        let parsed = cash_err!(
            oracle::parse_message(&payload),
            <Error<T>>::OpenOracleErrorInvalidMessage
        )?;

        let symbol = Symbols::get(parsed.key.clone()).expect("xxx our pattern for expect or err?");
        // XXX <Error<T>>::OpenOracleErrorInvalidSymbol

        debug::native::info!(
            "Parsed price from open price feed: {:?} is worth {:?}",
            parsed.key,
            (parsed.value as f64) / 1000000.0
        );
        // todo: more sanity checking on the value
        // note - the API for GET does not return an Option but if the value is not present it
        // returns Default::default(). Thus, we explicitly check if the key for the ticker is supported
        // or not.

        if PriceTimes::contains_key(&symbol) {
            // it has been updated at some point, make sure we are updating to a more recent price
            let last_updated = PriceTimes::get(&symbol);
            if parsed.timestamp <= last_updated {
                return Err(<Error<T>>::OpenOracleErrorStalePrice);
            }
        }

        // WARNING begin storage - all checks must happen above

        Prices::insert(&symbol, parsed.value as u128);
        PriceTimes::insert(&symbol, parsed.timestamp as u128);

        // todo: update storage
        Ok(())
    }

    /// Convert from the chain spec file format hex encoded strings into the local storage format, binary blob
    fn hex_string_vec_to_binary_vec<U: TryFrom<Vec<u8>>>(
        data: Vec<String>,
    ) -> Result<Vec<U>, Error<T>> {
        let mut converted: Vec<U> = Vec::new();
        for record in data {
            let decoded = hex::decode(&record).map_err(|_| <Error<T>>::GenesisConfigError)?;
            let converted_validator: U = decoded
                .try_into()
                .map_err(|_| <Error<T>>::GenesisConfigError)?;
            converted.push(converted_validator);
        }
        Ok(converted)
    }

    /// Set the initial set of validators from the genesis config
    fn initialize_validators(validators: ConfigSetString) {
        assert!(
            !validators.is_empty(),
            "Validators must be set in the genesis config"
        );
        assert!(
            Validators::get().is_empty(),
            "Validators are already set but they should only be set in the genesis config."
        );
        let converted: ValidatorSet = Self::hex_string_vec_to_binary_vec(validators)
            .expect("Could not deserialize validators from genesis config");
        Validators::put(converted);
    }

    // XXX actually symbols is a map with decimals
    /// Set the initial set of supported symbols from the genesis config
    fn initialize_symbols(symbols: ConfigSetString) {
        assert!(
            !symbols.is_empty(),
            "Symbols must be set in the genesis config"
        );

        for (ticker, decimals) in vec![("ETH", 18), ("USDC", 6)] {
            // XXX
            let mut chars: Vec<char> = ticker.chars().collect();
            chars.resize(12, 0 as char);
            let raw: [char; 12] = chars.try_into().expect("bad ticker");
            Symbols::insert(ticker, Symbol(raw, decimals));
        }
    }

    /// Set the initial set of open price feed price reporters from the genesis config
    fn initialize_reporters(reporters: ConfigSetString) {
        assert!(
            !reporters.is_empty(),
            "Open price feed price reporters must be set in the genesis config"
        );
        assert!(
            Reporters::get().is_empty(),
            "Reporters are already set but they should only be set in the genesis config."
        );
        let converted: Vec<[u8; 20]> = Self::hex_string_vec_to_binary_vec(reporters)
            .expect("Could not deserialize validators from genesis config");
        Reporters::put(converted);
    }

    /// Set the initial set of open price feed price reporters from the genesis config
    fn initialize_price_key_mapping(price_key_mapping: ConfigSetString) {
        assert!(
            !price_key_mapping.is_empty(),
            "price_key_mapping must be set in the genesis config"
        );

        for record in price_key_mapping {
            let mut tokens = record.split(":");
            let ticker = tokens.next().expect("No ticker present");
            let symbol = Symbols::get(ticker).expect("Ticker not a valid symbol");
            let chain = tokens.next().expect("No blockchain present");
            let address = tokens.next().expect("No address present");
            assert!(
                tokens.next().is_none(),
                "Too many colons in price key mapping element"
            );
            assert!(chain == "ETH", "Invalid blockchain");

            let decoded = hex::decode(&address).expect("Address is not in hex format");
            let decoded_arr: <Ethereum as Chain>::Address =
                decoded.try_into().expect("Invalid Ethereum address");

            let chain_asset: ChainAsset = ChainAsset::Eth(decoded_arr);
            PriceKeyMapping::insert(chain_asset, symbol);
        }
    }

    /// Procedure for offchain worker to processes messages coming out of the open price feed
    pub fn process_open_price_feed(block_number: T::BlockNumber) -> Result<(), Error<T>> {
        let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK_OPEN_PRICE_FEED);
        if lock.try_lock().is_err() {
            // working in another thread, no big deal
            return Ok(());
        }
        // check to see if it is time to poll or not
        let latest_price_feed_poll_block_number_storage =
            StorageValueRef::persistent(OCW_LATEST_PRICE_FEED_POLL_BLOCK_NUMBER);
        if let Some(Some(latest_poll_block_number)) =
            latest_price_feed_poll_block_number_storage.get::<T::BlockNumber>()
        {
            let poll_interval_blocks = <T as frame_system::Config>::BlockNumber::from(
                params::OCW_OPEN_ORACLE_POLL_INTERVAL_BLOCKS,
            );
            if block_number - latest_poll_block_number < poll_interval_blocks {
                return Ok(());
            }
        }
        // poll
        let api_response = cash_err!(
            crate::oracle::open_price_feed_request_okex(),
            <Error<T>>::OpenOracleHttpFetchError
        )?;
        let messages_and_signatures = cash_err!(
            api_response.to_message_signature_pairs(),
            <Error<T>>::OpenOracleApiResponseHexError
        )?;
        for (msg, sig) in messages_and_signatures {
            // adding some debug info in here, this will become very chatty
            let call = <Call<T>>::post_price(msg, sig);
            let _ = cash_err!(
                SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()),
                <Error<T>>::OpenOraclePostPriceExtrinsicError
            );
            // note - there is a log message in cash_err if this extrinsic fails but we should
            // still try to update the other prices even if one extrinsic fails, thus the result
            // is ignored and we continue in this loop
        }
        latest_price_feed_poll_block_number_storage.set(&block_number);
        Ok(())
    }

    pub fn process_eth_notices(block_number: T::BlockNumber) {
        for notice in EthNoticeQueue::iter_values() {
            // find parent
            // id = notice.gen_id(parent)

            // XXX
            // submit onchain call for aggregating the price
            // let payload = notices::to_payload(notice);
            // let call = Call::publish_eth_signature(payload);

            // Unsigned tx
            // SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
        }
    }

    // XXX disambiguate whats for ethereum vs not
    fn fetch_events_with_lock() -> Result<(), Error<T>> {
        // Create a reference to Local Storage value.
        // Since the local storage is common for all offchain workers, it's a good practice
        // to prepend our entry with the pallet name.
        let s_info = StorageValueRef::persistent(OCW_LATEST_CACHED_ETHEREUM_BLOCK);

        // Local storage is persisted and shared between runs of the offchain workers,
        // offchain workers may run concurrently. We can use the `mutate` function to
        // write a storage entry in an atomic fashion.
        //
        // With a similar API as `StorageValue` with the variables `get`, `set`, `mutate`.
        // We will likely want to use `mutate` to access
        // the storage comprehensively.
        //
        // Ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/storage/struct.StorageValueRef.html

        // XXXX TODO Add second check for:
        // Should be either the block of the latest Pending event which we haven't signed,
        // or if there are no such events, otherwise the block after the latest event, otherwise the earliest (configuration/genesis) block
        let from_block: String;
        if let Some(Some(cached_block_num)) = s_info.get::<String>() {
            // Ethereum block number has been cached, fetch events starting from the next after cached block
            debug::native::info!("Last cached block number: {:?}", cached_block_num);
            from_block = events::get_next_block_hex(cached_block_num)
                .map_err(|_| <Error<T>>::HttpFetchingError)?;
        } else {
            // Validator's cache is empty, fetch events from the earliest block with pending events
            debug::native::info!("Block number has not been cached yet");
            let block_numbers: Vec<u32> = EthEventQueue::iter()
                .filter_map(|((block_number, log_index), status)| {
                    if match status {
                        EventStatus::<Ethereum>::Pending { signers } => true,
                        _ => false,
                    } {
                        Some(block_number)
                    } else {
                        None
                    }
                })
                .collect();
            let pending_events_block = block_numbers.iter().min();
            if (pending_events_block.is_some()) {
                let events_block: u32 = *pending_events_block.unwrap();
                from_block = events::get_next_block_hex(events_block.to_string())
                    .map_err(|_| <Error<T>>::HttpFetchingError)?;
            } else {
                from_block = String::from("earliest");
            }
        }

        // Since off-chain storage can be accessed by off-chain workers from multiple runs, it is important to lock
        //   it before doing heavy computations or write operations.
        // ref: https://substrate.dev/rustdocs/v2.0.0-rc3/sp_runtime/offchain/storage_lock/index.html
        //
        // There are four ways of defining a lock:
        //   1) `new` - lock with default time and block expiration
        //   2) `with_deadline` - lock with default block but custom time expiration
        //   3) `with_block_deadline` - lock with default time but custom block expiration
        //   4) `with_block_and_time_deadline` - lock with custom time and block expiration
        // Here we choose the default one for now.
        let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK_ETHEREUM_EVENTS);

        // We try to acquire the lock here. If failed, we know the `fetch_n_parse` part inside is being
        //   executed by previous run of ocw, so the function just returns.
        // ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/storage_lock/struct.StorageLock.html#method.try_lock
        if let Ok(_guard) = lock.try_lock() {
            match events::fetch_events(from_block) {
                Ok(starport_info) => {
                    debug::native::info!("Result: {:?}", starport_info);

                    // Send extrinsics for all events
                    let _ = Self::process_lock_events(starport_info.lock_events)?;

                    // Save latest block in ocw storage
                    s_info.set(&starport_info.latest_eth_block);
                }
                Err(err) => {
                    debug::native::info!("Error while fetching events: {:?}", err);
                    return Err(Error::<T>::HttpFetchingError);
                }
            }
        }
        Ok(())
    }

    fn process_lock_events(
        events: Vec<ethereum_client::LogEvent<ethereum_client::LockEvent>>,
    ) -> Result<(), Error<T>> {
        for event in events.iter() {
            debug::native::info!("Processing `Lock` event and sending extrinsic: {:?}", event);

            // XXX
            let payload =
                events::to_lock_event_payload(&event).map_err(|_| <Error<T>>::HttpFetchingError)?;
            let signature = chains::eth::sign(&payload);
            let call = Call::process_eth_event(payload, signature);

            // XXX Unsigned tx for now
            let res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
            if res.is_err() {
                debug::native::error!("Error while sending `Lock` event extrinsic");
                return Err(Error::<T>::OffchainUnsignedLockTxError);
            }
        }
        Ok(())
    }
}

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    /// Validate unsigned call to this module.
    ///
    /// By default unsigned transactions are disallowed, but implementing the validator
    /// here we make sure that some particular calls (the ones produced by offchain worker)
    /// are being whitelisted and marked as valid.
    fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
        match call {
            Call::process_eth_event(payload, signature) => {
                ValidTransaction::with_tag_prefix("CashPallet")
                    .priority(100)
                    // The transaction is only valid for next 10 blocks. After that it's
                    // going to be revalidated by the pool.
                    .longevity(10)
                    .and_provides(payload)
                    // It's fine to propagate that transaction to other peers, which means it can be
                    // created even by nodes that don't produce blocks.
                    // Note that sometimes it's better to keep it for yourself (if you are the block
                    // producer), since for instance in some schemes others may copy your solution and
                    // claim a reward.
                    .propagate(true)
                    .build()
            }
            Call::exec_trx_request(request, signature) => {
                ValidTransaction::with_tag_prefix("CashPallet")
                    .longevity(10)
                    .and_provides(request)
                    .propagate(true)
                    .build()
            }
            Call::post_price(_, sig) => ValidTransaction::with_tag_prefix("CashPallet")
                .longevity(10)
                .and_provides(sig)
                .propagate(true)
                .build(),
            _ => InvalidTransaction::Call.into(),
        }
    }
}

// XXX this function is temporary and will be deleted after we reach a certain point
pub fn magic_extract_internal<T: Config>(
    sender: ChainAccount,
    account: ChainAccount,
    max_amount: MaxableAssetAmount,
) -> Result<(), Error<T>> {
    let _ = runtime_interfaces::config_interface::get(); // XXX where are we actually using this?

    let amount = get_max_value(max_amount, &|| 5);

    // Update storage -- TODO: increment this-- sure why not?
    let curr_cash_balance: AssetAmount = <CashBalance>::get(&account).unwrap_or_default();
    let next_cash_balance: AssetAmount = curr_cash_balance + amount;
    <CashBalance>::insert(&account, next_cash_balance);

    let now = <frame_system::Module<T>>::block_number().saturated_into::<u8>();
    // Add to Notice Queue
    // Note: we need to generalize this since it's not guaranteed to be Eth
    let notice_id = NoticeId(0, 0); // XXX need to keep state of current gen/within gen for each, also parent
                                    // TODO: This asset needs to match the chain-- which for Cash, there might be different
                                    //       versions of a given asset per chain
    let notice = Notice::CashExtractionNotice(match account {
        ChainAccount::Eth(eth_account) => CashExtractionNotice::Eth {
            id: notice_id,
            parent: [0u8; 32],
            account: eth_account,
            amount,
            cash_yield_index: 1000,
        },
        _ => return Err(Error::<T>::InvalidNoticeParams),
    });

    // TODO: Why is this Eth notice queue? Should this be specific to Eth?
    EthNoticeQueue::insert(
        notice_id,
        NoticeStatus::Pending {
            signers: vec![],
            signatures: ChainSignatureList::Eth(vec![]),
            notice: notice.clone(),
        },
    );

    let encoded_notice = notice.encode_ethereum_notice();
    let signature = chains::eth::sign(&encoded_notice[..]);

    // Emit an event or two.
    <Module<T>>::deposit_event(RawEvent::MagicExtract(amount, account, notice.clone()));
    <Module<T>>::deposit_event(RawEvent::SignedNotice(
        ChainId::Eth,
        notice_id,
        encoded_notice,
        ChainSignatureList::Eth(vec![signature]),
    )); // XXX signatures

    // Return a successful DispatchResult
    Ok(())
}
