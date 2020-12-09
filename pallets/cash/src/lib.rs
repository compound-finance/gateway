#![cfg_attr(not(feature = "std"), no_std)]

use codec::alloc::string::String;
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get,
};
use frame_system::ensure_signed;
use sp_std::vec::Vec;

use sp_runtime::offchain::{
    storage::StorageValueRef,
    storage_lock::{StorageLock, Time},
};

extern crate ethereum_client;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[macro_use]
extern crate alloc;

pub const ETH_STARPORT_ADDRESS: &str = "0xbbde1662bC3ED16aA8C618c9833c801F3543B587";
pub const LOCK_EVENT_TOPIC: &str =
    "0xec36c0364d931187a76cf66d7eee08fad0ec2e8b7458a8d8b26b36769d4d13f3";

// OCW storage constants
pub const OCW_STORAGE_LOCK_KEY: &[u8; 10] = b"cash::lock";
pub const OCW_LATEST_CACHED_BLOCK: &[u8; 11] = b"cash::block";
// pub const LOCK_TIMEOUT_EXPIRATION: u64 = FETCH_TIMEOUT_PERIOD + 1000; // in milli-seconds
// pub const LOCK_BLOCK_EXPIRATION: u32 = 3; // in block number

#[derive(Debug)]
struct StarportInfo {
    latest_eth_block: String,
    lock_events: Vec<ethereum_client::LogEvent<ethereum_client::LockEvent>>,
}

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

decl_storage! {
    trait Store for Module<T: Config> as CashPalletModule {
        // XXX
        // Learn more about declaring storage items:
        // https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
        Something get(fn something): Option<u32>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
    {
        // XXX
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        SomethingStored(u32, AccountId),
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
        pub fn do_something(origin, something: u32) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;

            // Update storage.
            Something::put(something);

            // Emit an event.
            Self::deposit_event(RawEvent::SomethingStored(something, who));
            // Return a successful DispatchResult
            Ok(())
        }

        /// An example dispatchable that may throw a custom error.
        #[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
        pub fn cause_error(origin) -> dispatch::DispatchResult {
            let _who = ensure_signed(origin)?;
            let _ = runtime_interfaces::config_interface::get();

            // Read a value from storage.
            match Something::get() {
                // Return an error if the value has not been set.
                None => Err(Error::<T>::NoneValue)?,
                Some(old) => {
                    // Increment the value read from storage; will error in the event of overflow.
                    let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
                    // Update the value in storage with the incremented result.
                    Something::put(new);
                    Ok(())
                },
            }
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
        }
    }
}

/// Most of the functions are moved outside of the `decl_module!` macro.
///
/// This greatly helps with error messages, as the ones inside the macro
/// can sometimes be hard to debug.
impl<T: Config> Module<T> {
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
        let from_block: String;
        if let Some(Some(cached_block_num)) = s_info.get::<String>() {
            // Ethereum block number has been cached, fetch events starting from the next after cached block
            debug::native::info!("Last cached block number: {:?}", cached_block_num);
            from_block = Self::get_next_block_hex(cached_block_num)
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
        // Here we choose the most custom one for demonstration purpose.
        let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK_KEY);

        // We try to acquire the lock here. If failed, we know the `fetch_n_parse` part inside is being
        //   executed by previous run of ocw, so the function just returns.
        // ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/storage_lock/struct.StorageLock.html#method.try_lock
        if let Ok(_guard) = lock.try_lock() {
            match Self::fetch_events(eth_rpc_url, from_block) {
                Ok(starport_info) => {
                    debug::native::info!("Result: {:?}", starport_info);
                    s_info.set(&starport_info.latest_eth_block);
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        Ok(())
    }

    /// Fetch all latest Starport events for the offchain worker.
    fn fetch_events(eth_rpc_url: String, from_block: String) -> Result<StarportInfo, Error<T>> {
        // Fetch the latest available ethereum block number
        let latest_eth_block = ethereum_client::fetch_latest_block(&eth_rpc_url).map_err(|e| {
            debug::native::error!("fetch_events error: {:?}", e);
            <Error<T>>::HttpFetchingError
        })?;

        // Build parameters set for fetching starport `Lock` events
        let fetch_events_request = format!(
            r#"{{"address": "{}", "fromBlock": "{}", "toBlock": "{}", "topics":["{}"]}}"#,
            ETH_STARPORT_ADDRESS, from_block, latest_eth_block, LOCK_EVENT_TOPIC
        );

        // Fetch `Lock` events using ethereum_client
        let lock_events =
            ethereum_client::fetch_and_decode_events(&eth_rpc_url, vec![&fetch_events_request])
                .map_err(|_| <Error<T>>::HttpFetchingError)?;

        Ok(StarportInfo {
            lock_events: lock_events,
            latest_eth_block: latest_eth_block,
        })
    }

    fn get_next_block_hex(block_num_hex: String) -> Result<String, Error<T>> {
        let without_prefix = block_num_hex.trim_start_matches("0x");
        let block_num =
            u64::from_str_radix(without_prefix, 16).map_err(|_| <Error<T>>::HttpFetchingError)?;
        let next_block_num_hex = format!("{:#X}", block_num + 1);
        Ok(next_block_num_hex)
    }
}
