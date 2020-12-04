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

pub const ETHEREUM_STARPORT_ADDRESS: &str = "0xbbde1662bC3ED16aA8C618c9833c801F3543B587";
pub const LOCK_EVENT_TOPIC: &str =
    "0xec36c0364d931187a76cf66d7eee08fad0ec2e8b7458a8d8b26b36769d4d13f3";
pub const OCW_LOCK_ID: &[u8; 12] = b"access::lock";
pub const OCW_LATEST_FETCHED_BLOCK_ID: &[u8; 29] = b"offchain-worker::latest-block";

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
            let config = runtime_interfaces::config_interface::get();
            let eth_rpc_url = String::from_utf8(config.get_eth_rpc_url()).unwrap();
            // debug::native::info!("CONFIG: {:?}", eth_rpc_url);

            let mut lock = StorageLock::<Time>::new(OCW_LOCK_ID);
            {
                let _guard = lock.lock();
                let acc = StorageValueRef::persistent(OCW_LATEST_FETCHED_BLOCK_ID);
                let from_block : String;
                if let Some(Some(latest_block)) = acc.get::<u64>() {
                    debug::native::info!("cached info: {:?}", latest_block);
                    from_block = format!("{:#X}", latest_block + 1);
               } else {
                   debug::native::info!("No latest block was found in the cash");
                   from_block = String::from("earliest");
               }

               let fetch_events_request = format!(r#"{{"address": "{}", "fromBlock": "{}", "toBlock": "latest", "topics":["{}"]}}"#, ETHEREUM_STARPORT_ADDRESS, from_block, LOCK_EVENT_TOPIC);
               let (end_block, lock_events) : (i64, Vec<ethereum_client::LogEvent<ethereum_client::LockEvent>>) = ethereum_client::fetch_and_decode_events(&eth_rpc_url, vec![&fetch_events_request]).unwrap();
               debug::native::info!("end_block: {:?}", end_block);
               debug::native::info!("Lock Events: {:?}", lock_events);

               // Set the latest block if at least one event was retrieved
               if end_block != 0 {
                   acc.set(&end_block);
               }
               // drop `_guard` implicitly at end of scope
            }
        }
    }
}
