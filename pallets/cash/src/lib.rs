#![allow(incomplete_features)]
#![feature(associated_type_defaults)]
#![feature(const_generics)]
#![feature(array_methods)]

#[macro_use]
extern crate alloc;
extern crate ethereum_client;

use codec::{alloc::string::String, Decode};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get,
};
use frame_system::{
    ensure_none, ensure_signed,
    offchain::{CreateSignedTransaction, SubmitTransaction},
};
use our_std::{convert::TryInto, vec::Vec};
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
use crate::core::{
    Account, Asset, EventStatus, GenericAccount, GenericAsset, GenericMsg, GenericPrice,
    GenericQty, GenericSigs, Index, Nonce, NoticeStatus, Reason, SignedPayload, Symbol, Timestamp,
    ValidatorGenesisConfig, ValidatorSet, ValidatorSig, APR,
};
use crate::notices::{Notice, NoticeId}; // XXX move to core?
use our_std::convert::TryFrom;

mod chains;
mod core;
mod events;
mod notices;
mod params;

#[cfg(test)]
mod mock;

mod oracle;
#[cfg(test)]
mod tests;

/// Type for open price feed reporter set included in the genesis config
pub type ReporterGenesisConfig = Vec<String>;

// OCW storage constants
pub const OCW_STORAGE_LOCK_KEY: &[u8; 10] = b"cash::lock";
pub const OCW_LATEST_CACHED_BLOCK: &[u8; 11] = b"cash::block";

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
        CashCostIndex get(fn cash_cost_index): Index;

        /// An index to track interest earned by CASH holders.
        CashYieldIndex get(fn cash_yield_index): Index;

        /// The upcoming base rate change for CASH and when, if any.
        CashYieldNext get(fn cash_yield_next): Option<(APR, Timestamp)>;

        /// The current APR on CASH held, and the base rate paid by borrowers.
        CashYield get(fn cash_yield): APR;

        /// The current APR surcharge on CASH borrowed, added to the CashYield to determine the CashCost.
        CashSpread get(fn cash_spread): APR;

        /// An index to track interest owed by asset borrowers.
        BorrowIndex get(fn borrow_index): map hasher(blake2_128_concat) GenericAsset => Index;

        /// An index to track interest earned by asset suppliers.
        SupplyIndex get(fn supply_index): map hasher(blake2_128_concat) GenericAsset => Index;

        // XXX doc
        ChainCashHoldPrincipal get(fn chain_cash_hold_principal): map hasher(blake2_128_concat) ChainId => GenericQty;
        // TotalCashHoldPrincipal;
        // TotalCashBorrowPrincipal;
        // TotalSupplyPrincipal[asset];
        // TotalBorrowPrincipal[asset];
        // CashHoldPrincipal[account];
        // CashBorrowPrincipal[account];
        // SupplyPrincipal[asset][account];
        // BorrowPrincipal[asset][account];

        /// The last used nonce for each account, initialized at zero.
        Nonces get(fn nonces): map hasher(blake2_128_concat) GenericAccount => Nonce;

        // XXX delete me (part of magic extract)
        CashBalance get(fn cash_balance): map hasher(blake2_128_concat) GenericAccount => Option<GenericQty>;

        /// Mapping of (status of) events witnessed on Ethereum, by event id.
        EthEventQueue get(fn eth_event_queue): map hasher(blake2_128_concat) chains::eth::EventId => Option<EventStatus<Ethereum>>;

        /// Mapping of (status of) notices to be signed for Ethereum, by notice id.
        EthNoticeQueue get(fn eth_notice_queue): map hasher(blake2_128_concat) NoticeId => Option<NoticeStatus<Ethereum>>;

        /// Mapping of assets to their price.
        Price get(fn price): map hasher(blake2_128_concat) GenericAsset => GenericQty;

        /// Mapping of assets to the last time their price was updated.
        PriceTime get(fn price_time): map hasher(blake2_128_concat) GenericAsset => Timestamp;

        /// Mapping from exchange ticker ("USDC") to GenericAsset - note this changes based on testnet/mainnet
        PriceKeyMapping get(fn price_key_mapping): map hasher(blake2_128_concat) String => GenericAsset;

        /// Eth addresses of price reporters for open oracle
        Reporters get(fn reporters): Vec<[u8; 20]>;

        // XXX
        // AssetInfo[asset];
        // LiquidationIncentive;
        // PriceReporter;
        // PriceKeyMapping;
    }
    add_extra_genesis {
        config(validators): ValidatorGenesisConfig;
        config(reporters): ReporterGenesisConfig;
        build(|config| {
            Module::<T>::initialize_validators(config.validators.clone());
            Module::<T>::initialize_reporters(config.reporters.clone());
        })
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId, //XXX seems to require a where clause with reference to T to compile
    {
        XXXPhantomFakeEvent(AccountId), // XXX

        /// XXX
        MagicExtract(GenericQty, GenericAccount, Notice<Ethereum>),

        /// An Ethereum event was successfully processed. [payload]
        ProcessedEthEvent(SignedPayload),

        /// An Ethereum event failed during processing. [payload, reason]
        FailedProcessingEthEvent(SignedPayload, Reason),

        /// Signed Ethereum notice. [message, signatures]
        SignedNotice(ChainId, NoticeId, GenericMsg, GenericSigs),
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

        /// An error related to the chain_spec file contents
        GenesisConfigError,
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

        // XXX this function is temporary and will be deleted after we reach a certain point
        /// An example dispatchable that takes a singles value as a parameter, writes the value to
        /// storage and emits an event. This function must be dispatched by a signed extrinsic.
        #[weight = 10_000 + T::DbWeight::get().writes(1)]
        pub fn magic_extract(origin, account: GenericAccount, amount: GenericQty) -> dispatch::DispatchResult {
            ensure_none(origin)?;
            let _ = runtime_interfaces::config_interface::get(); // XXX where are we actually using this?

            // Update storage -- TODO: increment this-- sure why not?
            let curr_cash_balance: GenericQty = <CashBalance>::get(&account).unwrap_or_default();
            let next_cash_balance: GenericQty = curr_cash_balance + amount;
            <CashBalance>::insert(&account, next_cash_balance);

            let now = <frame_system::Module<T>>::block_number().saturated_into::<u8>();
            // Add to Notice Queue
            let notice = Notice::ExtractionNotice {
                id: NoticeId(now.into(), 0),  // XXX need to keep state of current gen/within gen for each, also parent
                parent: [0u8; 32], // XXX,
                asset: Asset([0u8; 20]),
                amount: amount.try_into()
                    .unwrap_or_else(|_| panic!("Amount does not fit on chain")),
                account: Account(account.1.clone().try_into() // XXX avoid clone?
                    .unwrap_or_else(|_| panic!("Address has wrong number of bytes")))
            };
            EthNoticeQueue::insert(notice.id(), NoticeStatus::<Ethereum>::Pending { signers: vec![], signatures: vec![], notice: notice.clone()});

            // Emit an event.
            Self::deposit_event(RawEvent::SignedNotice(ChainId::Eth, notice.id(), notices::encode_ethereum_notice(notice), vec![])); // XXX signatures
            // Return a successful DispatchResult
            Ok(())
        }

        #[weight = 0] // XXX how are we doing weights?
        pub fn process_eth_event(origin, payload: SignedPayload, sig: ValidatorSig) -> dispatch::DispatchResult { // XXX sig
            // XXX do we want to store/check hash to allow replaying?
            //let signer = recover(payload); // XXX

            // XXX how do we want to deal with these crypto errors?
            // XXX Toni WIP
            // Recover signature part
            let mut sig_part: [u8; 64] = [0; 64];
            sig_part[0..64].copy_from_slice(&sig[0..64]);
            let signature = secp256k1::Signature::parse(&sig_part);
            // Recover RecoveryId part
            let recovery_id = cash_err!(secp256k1::RecoveryId::parse(sig[64]), <Error<T>>::SignedPayloadError)?;
            // Recover validator's public key from signature
            let message = secp256k1::Message::parse(&chains::eth::keccak(payload.clone()));
            let recover = secp256k1::recover(&message, &signature, &recovery_id).map_err(|_| <Error<T>>::SignedPayloadError)?;
            let signer = recover.serialize();

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
                        match core::apply_eth_event_internal(event) {
                            Ok(_) => {
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
                    match core::apply_eth_event_internal(event) {
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

            let status = <EthNoticeQueue>::get(notice_id).unwrap_or(NoticeStatus::<Ethereum>::Missing);
            match status {
                NoticeStatus::<Ethereum>::Missing => {
                    Ok(())
                }

                NoticeStatus::<Ethereum>::Pending { signers, signatures, notice } => {
                    // XXX sets?
                    // let signers_new = {signer | signers};
                    // let signatures_new = {signature | signatures };
                    // if len(signers_new & Validators) > 2/3 * len(Validators) {
                           EthNoticeQueue::insert(notice_id, NoticeStatus::<Ethereum>::Done);
                           Self::deposit_event(RawEvent::SignedNotice(ChainId::Eth, notice_id, notices::encode_ethereum_notice(notice), signatures)); // XXX generic
                           Ok(())
                    // } else {
                    //     EthNoticeQueue::insert(event.id, NoticeStatus::<Ethereum>::Pending { signers: signers_new, signatures, notice});
                    //     Ok(())
                    // }
                }

                NoticeStatus::<Ethereum>::Done => {
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
            let result = Self::process_open_price_feed();
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
            <Error<T>>::OpenOracleError
        )?;
        let reporters = Reporters::get();
        if !reporters
            .iter()
            .any(|e| e.as_slice() == recovered.as_slice())
        {
            return Err(<Error<T>>::OpenOracleError);
        }

        let parsed = cash_err!(oracle::parse_message(&payload), <Error<T>>::OpenOracleError)?;

        debug::native::info!(
            "Parsed price from open price feed: {:?} is worth {:?}",
            parsed.key,
            (parsed.value as f64) / 1000000.0
        );
        // todo: more sanity checking on the value
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
    fn initialize_validators(validators: ValidatorGenesisConfig) {
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

    /// Set the initial set of open price feed price reporters from the genesis config
    fn initialize_reporters(reporters: ReporterGenesisConfig) {
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

    /// Procedure for offchain worker to processes messages coming out of the open price feed
    pub fn process_open_price_feed() -> Result<(), Error<T>> {
        let api_response = cash_err!(
            crate::oracle::open_price_feed_request_okex(),
            <Error<T>>::HttpFetchingError
        )?;
        let messages_and_signatures = cash_err!(
            api_response.to_message_signature_pairs(),
            <Error<T>>::OpenOracleError
        )?;
        for (msg, sig) in messages_and_signatures {
            // adding some debug info in here, this will become very chatty
            let call = <Call<T>>::post_price(msg, sig);
            cash_err!(
                SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()),
                <Error<T>>::OpenOracleError
            )?;
        }
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
        let s_info = StorageValueRef::persistent(OCW_LATEST_CACHED_BLOCK);

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
            // Validator's cache is empty, fetch events from the earliest available block
            debug::native::info!("Block number has not been cached yet");
            from_block = String::from("earliest");
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
        let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK_KEY);

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
            let signature = Self::sign_payload(payload.clone());
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

    // XXX JF: why are we using secp256k1 to sign these? this is just for offchain <> validator?
    /// XXX this part will be rewritten as soon as keys are done
    fn sign_payload(payload: Vec<u8>) -> ValidatorSig {
        // XXX HORRIBLE, but something to move on, at least I can decode signature
        let not_so_secret: [u8; 32] =
            hex_literal::hex!["50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374"];
        let private_key = secp256k1::SecretKey::parse(&not_so_secret).unwrap();
        let message = secp256k1::Message::parse(&chains::eth::keccak(payload.clone()));
        let sig = secp256k1::sign(&message, &private_key);
        let mut r: [u8; 65] = [0; 65];
        r[0..64].copy_from_slice(&sig.0.serialize()[..]);
        r[64] = sig.1.serialize();
        return r;
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
            Call::process_eth_event(_payload, signature) => {
                ValidTransaction::with_tag_prefix("CashPallet")
                    // The transaction is only valid for next 10 blocks. After that it's
                    // going to be revalidated by the pool.
                    .longevity(10)
                    .and_provides(signature)
                    // It's fine to propagate that transaction to other peers, which means it can be
                    // created even by nodes that don't produce blocks.
                    // Note that sometimes it's better to keep it for yourself (if you are the block
                    // producer), since for instance in some schemes others may copy your solution and
                    // claim a reward.
                    .propagate(true)
                    .build()
            }
            Call::post_price(_, _) => ValidTransaction::with_tag_prefix("CashPallet")
                .longevity(10)
                .propagate(true)
                .build(),
            _ => InvalidTransaction::Call.into(),
        }
    }
}
