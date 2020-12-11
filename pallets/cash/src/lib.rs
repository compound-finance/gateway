#![cfg_attr(not(feature = "std"), no_std)]

use codec::{alloc::string::String, Decode, Encode};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get,
};

use crate::account::{AccountAddr, AccountIdent, ChainIdent};
use crate::amount::{Amount, CashAmount};
use crate::notices::{EthHash, Notice};

use frame_system::{
    ensure_none, ensure_signed,
    offchain::{CreateSignedTransaction, SubmitTransaction},
};
use sp_runtime::RuntimeDebug;
use sp_runtime::{
    offchain::http,
    transaction_validity::{TransactionSource, TransactionValidity, ValidTransaction},
};
use sp_std::vec::Vec;

use sp_runtime::offchain::{
    storage::StorageValueRef,
    storage_lock::{StorageLock, Time},
};
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};

extern crate ethereum_client;

mod chains;
mod core;

#[cfg(test)]
mod mock;

mod account;
mod amount;
mod events;
mod notices;

#[cfg(test)]
mod tests;

#[macro_use]
extern crate alloc;

// XXX move / unrely
pub type Hash = Vec<u8>;

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Reason {
    None,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum EthEventStatus {
    Pending { signers: Vec<u8> },          // XXX set(Public)?
    Failed { hash: Hash, reason: Reason }, // XXX type for err reasons?
    Done,
}

// XXXX experimental
pub fn compute_hash(payload: chains::eth::Payload) -> Hash {
    // XXX where Payload: Hash
    payload // XXX
}

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
        // XXX
        CashBalance get(fn cash_balance) config(): map hasher(blake2_128_concat) AccountIdent => Option<CashAmount>;

        // Mapping of (status of) events witnessed on Ethereum by event id.
        EthEventStatuses get(fn eth_event_statuses): map hasher(twox_64_concat) chains::eth::EventId => Option<EthEventStatus>;

        // TODO: hash type should match to ChainIdent
        pub NoticeQueue get(fn notice_queue): double_map hasher(blake2_128_concat) ChainIdent, hasher(blake2_128_concat) EthHash => Option<Notice>;

    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId, //XXX seems to require a where clause with reference to T to compile
        Payload = chains::eth::Payload,                     //XXX<T as Config>::Payload ?,
    {
        XXXPhantomFakeEvent(AccountId), // XXX

        /// XXX
        MagicExtract(CashAmount, AccountIdent, Notice),
        Notice(notices::Message, notices::Signature, AccountIdent),

        /// An Ethereum event was successfully processed. [payload]
        ProcessedEthereumEvent(Payload),

        /// An Ethereum event failed during processing. [payload, reason]
        FailedProcessingEthereumEvent(Payload, Reason),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        // XXX
        /// Error names should be descriptive.
        NoneValue,
        /// Errors should have helpful documentation associated with them.
        StorageOverflow,

        // Error returned when fetching starport info
        HttpFetchingError,

        // Error when processing `Lock` event while sending `process_ethereum_event` extrinsic
        ProcessLockEventError,
    }
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

        /// An example dispatchable that takes a singles value as a parameter, writes the value to
        /// storage and emits an event. This function must be dispatched by a signed extrinsic.
        #[weight = 10_000 + T::DbWeight::get().writes(1)]
        pub fn magic_extract(origin, account: AccountIdent, amount: CashAmount) -> dispatch::DispatchResult {
            let () = ensure_none(origin)?;

            // Update storage -- TODO: increment this-- sure why not?
            let curr_cash_balance: CashAmount = CashBalance::get(&account).unwrap_or_default();
            let next_cash_balance: CashAmount = curr_cash_balance.checked_add(amount).ok_or(Error::<T>::StorageOverflow)?;
            CashBalance::insert(&account, next_cash_balance);

            // Add to Notice Queue
            let notice = Notice::ExtractionNotice {asset: Vec::new(), amount: Amount::new_cash(amount), account: account.clone()};
            let dummy_hash: [u8; 32] = [0; 32];
            NoticeQueue::insert(ChainIdent::Eth, dummy_hash, &notice);

            // Emit an event.
            Self::deposit_event(RawEvent::MagicExtract(amount, account, notice));
            // Return a successful DispatchResult
            Ok(())
        }

        #[weight = 0] // XXX how are we doing weights?
        pub fn process_ethereum_event(origin, payload: chains::eth::Payload) -> dispatch::DispatchResult {
            // XXX
            // XXX do we want to store/check hash to allow replaying?
            //let signer = recover(payload);
            //require(signer == known validator);
            let event = chains::eth::decode(payload.clone()); // XXX
            let status = <EthEventStatuses>::get(event.id).unwrap_or(EthEventStatus::Pending { signers: vec![] }); // XXX
            match status {
                EthEventStatus::Pending { signers } => {
                    // XXX sets?
                    // let signers_new = {signer | signers};
                    // if len(signers_new & Validators) > 2/3 * len(Validators) {
                        match core::apply_ethereum_event_internal(event) {
                            Ok(_) => {
                                EthEventStatuses::insert(event.id, EthEventStatus::Done);
                                Self::deposit_event(RawEvent::ProcessedEthereumEvent(payload));
                                Ok(())
                            }

                            Err(reason) => {
                                EthEventStatuses::insert(event.id, EthEventStatus::Failed { hash: compute_hash(payload.clone()), reason: reason.clone() }); // XXX better way?
                                Self::deposit_event(RawEvent::FailedProcessingEthereumEvent(payload, reason));
                                Ok(())
                            }
                        }
                    // } else {
                    //     EthEventStatuses::put(event.id, EthEventStatus::Pending(signers_new));
                    // }
                }

                // XXX potential retry logic to allow retrying Failures
                EthEventStatus::Failed { hash, reason } => {
                    //require(compute_hash(payload) == hash, "event data differs from failure");
                    match core::apply_ethereum_event_internal(event) {
                        Ok(_) => {
                            EthEventStatuses::insert(event.id, EthEventStatus::Done);
                            Self::deposit_event(RawEvent::ProcessedEthereumEvent(payload));
                            Ok(())
                        }

                        Err(new_reason) => {
                            EthEventStatuses::insert(event.id, EthEventStatus::Failed { hash, reason: new_reason.clone()}); // XXX
                            Self::deposit_event(RawEvent::FailedProcessingEthereumEvent(payload, new_reason));
                            Ok(())
                        }
                    }
                }

                EthEventStatus::Done => Ok(())
            }
        }

        // XXX
        /// An example dispatchable that may throw a custom error.
        #[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
        pub fn cause_error(origin) -> dispatch::DispatchResult {
            let _who = ensure_signed(origin)?;
            let _ = runtime_interfaces::config_interface::get();
            Err(Error::<T>::NoneValue)?
        }

        #[weight = 0]
        pub fn emit_notice(origin, notice: notices::NoticePayload) -> dispatch::DispatchResult {
            // TODO: Move to using unsigned and getting author from signature
            // TODO I don't know what this comment means ^
            ensure_none(origin)?;

            debug::native::info!("emitting notice: {:?}", notice);
            Self::deposit_event(RawEvent::Notice(notice.msg, notice.sig, notice.signer));

            Ok(())
        }

        /// Offchain Worker entry point.
        ///
        /// By implementing `fn offchain_worker` within `decl_module!` you declare a new offchain
        /// worker.
        /// This function will be called when the node is fully synced and a new best block is
        /// succesfuly imported.
        /// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
        /// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
        /// so the code should be able to handle that.
        /// You can use `Local Storage` API to coordinate runs of the worker.
        fn offchain_worker(block_number: T::BlockNumber) {
            debug::native::info!("Hello World from offchain workers from block {} !", block_number);

            let result = Self::fetch_events_with_lock();
            if let Err(e) = result {
                debug::native::error!("offchain_worker error: {:?}", e);
            }
            Self::process_notices(block_number);
        }
    }
}

/// Reading error messages inside `decl_module!` can be difficult, so we move them here.
impl<T: Config> Module<T> {
    pub fn process_notices(block_number: T::BlockNumber) {
        let n = notices::Notice::ExtractionNotice {
            asset: "eth:0xfffff".as_bytes().to_vec(),
            account: AccountIdent {
                chain: ChainIdent::Eth,
                account: "eth:0xF33d".as_bytes().to_vec(),
            },
            amount: Amount::new(2000_u32, 3),
        };

        // let pending_notices : Vec<Box<Notice>> =  vec![Box::new(n)];
        let pending_notices: Vec<Notice> = vec![n];

        for notice in pending_notices.iter() {
            // find parent
            // id = notice.gen_id(parent)

            // submit onchain call for aggregating the price
            let payload = notices::to_payload(notice);
            let call = Call::emit_notice(payload);

            // Unsigned tx
            SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
        }
    }

    fn fetch_events_with_lock() -> Result<(), Error<T>> {
        // Get a validator config from runtime-interfaces pallet
        // Use config to get an address for interacting with Ethereum JSON RPC client
        let config = runtime_interfaces::config_interface::get();
        let eth_rpc_url = String::from_utf8(config.get_eth_rpc_url())
            .map_err(|_| <Error<T>>::HttpFetchingError)?;

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
            match events::fetch_events(eth_rpc_url, from_block) {
                Ok(starport_info) => {
                    debug::native::info!("Result: {:?}", starport_info);

                    // Send extrinsics for all events
                    let _ = Self::process_lock_events(starport_info.lock_events);

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
            debug::native::info!(
                "Processing `Lock` event and sending extrinsics: {:?}",
                event
            );

            let payload = events::to_payload(&event).map_err(|_| <Error<T>>::HttpFetchingError)?;
            let call = Call::process_ethereum_event(payload);

            // Unsigned tx
            let _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
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
        // TODO: This is not ready for prime-time
        ValidTransaction::with_tag_prefix("CashPallet")
            // The transaction is only valid for next 10 blocks. After that it's
            // going to be revalidated by the pool.
            .longevity(10)
            // XXXX Figure out if we need it and why we need it???
            .priority(10)
            .and_provides("fix_this_function")
            // It's fine to propagate that transaction to other peers, which means it can be
            // created even by nodes that don't produce blocks.
            // Note that sometimes it's better to keep it for yourself (if you are the block
            // producer), since for instance in some schemes others may copy your solution and
            // claim a reward.
            .propagate(true)
            .build()
    }
}
