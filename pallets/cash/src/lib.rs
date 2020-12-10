#![cfg_attr(not(feature = "std"), no_std)]

use codec::{alloc::string::String, Decode, Encode};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get,
};
use frame_system::{ensure_none, ensure_signed};
use sp_runtime::{
    offchain::http,
    transaction_validity::{TransactionSource, TransactionValidity, ValidTransaction},
};
use sp_std::vec::Vec;

use sp_runtime::RuntimeDebug;
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};

use crate::account::{AccountIdent, ChainIdent};
use crate::amount::{Amount, CashAmount};
use crate::notices::{Notice, EthHash};

extern crate ethereum_client;

mod chains;
mod core;

#[cfg(test)]
mod mock;

mod account;
mod amount;
mod notices;

#[cfg(test)]
mod tests;

#[macro_use]
extern crate alloc;

// XXX move / unrely
pub type Hash = Vec<u8>;
pub type Payload = Vec<u8>;

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
pub fn compute_hash(payload: Payload) -> Hash {
    // XXX where Payload: Hash
    payload // XXX
}

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    // XXX what should be configurable?
    //type EthEventStatus: From<EthEventStatus> + Into<EthEventStatus>;
    //type Payload: From<Vec<u8>> + Into<Vec<u8>>;
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
        Payload = Payload,                                  //XXX<T as Config>::Payload ?,
    {
        XXXPhantomFakeEvent(AccountId), // XXX

        /// XXX
        MagicExtract(CashAmount, AccountIdent, Notice),

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
        pub fn process_ethereum_event(origin, payload: Payload) -> dispatch::DispatchResult {
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
            debug::native::info!("Hello World from offchain workers!");
            let config = runtime_interfaces::config_interface::get();
            let eth_rpc_url = String::from_utf8(config.get_eth_rpc_url()).unwrap();
            // debug::native::info!("CONFIG: {:?}", eth_rpc_url);

            // TODO create parameter vector from storage variables
            let lock_events: Result<Vec<ethereum_client::LogEvent<ethereum_client::LockEvent>>, http::Error> = ethereum_client::fetch_and_decode_events(&eth_rpc_url, vec!["{\"address\": \"0x3f861853B41e19D5BBe03363Bb2f50D191a723A2\", \"fromBlock\": \"0x146A47D\", \"toBlock\" : \"latest\", \"topics\":[\"0xddd0ae9ae645d3e7702ed6a55b29d04590c55af248d51c92c674638f3fb9d575\"]}"]);
            debug::native::info!("Lock Events: {:?}", lock_events);
            Self::process_notices();
        }
    }
}

/// Reading error messages inside `decl_module!` can be difficult, so we move them here.
impl<T: Config> Module<T> {
    pub fn process_notices() {
        // notice queue stub

        // let signer = Signer::<T, T::AuthorityId>::any_account();
        for notice in NoticeQueue::iter() {
        //     // find parent
        //     // id = notice.gen_id(parent)
        //     let message = notice.encode();
        //     signer.send_unsigned_transaction(
        //         |account| notices::NoticePayload {
        //             // id: move id,
        //             msg: message.clone(),
        //             sig: notices::sign(&message),
        //             public: account.public.clone(),
        //         },
        //         Call::emit_notice);
        // }

        }
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
