#![feature(associated_type_defaults)]
#![feature(const_generics)]

#[macro_use]
extern crate alloc;
extern crate ethereum_client;

use codec::{alloc::string::String, Decode, Encode};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get,
};
use frame_system::{
    ensure_none, ensure_signed,
    offchain::{CreateSignedTransaction, SubmitTransaction},
};
use hex_literal::hex;
use our_std::{convert::TryInto, vec::Vec};
use sp_runtime::{
    offchain::{
        storage::StorageValueRef,
        storage_lock::{StorageLock, Time},
    },
    transaction_validity::{TransactionSource, TransactionValidity, ValidTransaction},
    RuntimeDebug, SaturatedConversion,
};

use crate::account::AccountIdent;
use crate::amount::CashAmount;
use crate::chains::{Chain, Ethereum, EventStatus}; // XXX events mod?
use crate::notices::{Notice, NoticeId, NoticeStatus};

mod account;
mod amount;
mod chains;
mod core;
mod events;
mod notices; // XXX

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum Reason {
    None,
}

/// Type for an encoded payload within an extrinsic.
pub type SignedPayload = Vec<u8>; // XXX

/// Type for signature used to verify that a signed payload comes from a validator.
pub type ValidatorSig = [u8; 65]; // XXX secp256k1 sign, but why secp256k1?

/// Type for a public key used to identify a validator.
pub type ValidatorKey = [u8; 65]; // XXX secp256k1 public key, but why secp256k1?

/// Type for a set of validator identities.
pub type ValidatorSet = Vec<ValidatorKey>; // XXX whats our set type? ordered Vec?

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
        XXXFakeForGenesis get(fn xxx_fake_for_genesis) config(): u32;

        // XXX
        CashBalance get(fn cash_balance): map hasher(blake2_128_concat) AccountIdent => Option<CashAmount>;

        // Mapping of (status of) events witnessed on Ethereum, by event id.
        EthEventQueue get(fn eth_event_queue): map hasher(blake2_128_concat) chains::eth::EventId => Option<EventStatus<Ethereum>>;

        // Mapping of (status of) notices to be signed for Ethereum, by notice id.
        EthNoticeQueue get(fn eth_notice_queue): map hasher(blake2_128_concat) NoticeId => Option<NoticeStatus<Ethereum>>;

        // XXX
        // List of validators' public keys
        Validators get(fn validators): ValidatorSet = vec![
            hex!("0458bfa2eec1cd8f451b41a1ad1034614986a6e65eabe24b5a7888d3f7422d6130e35d36561b207b1f9462bd8a982bd5b5204a2f8827b38469841ef537554ff1ba"),
            hex!("04c3e5ff2cb194d58e6a51ffe2df490c70d899fee4cdfff0a834fcdfd327a1d1bdaae3f1719d7fd9a9ee4472aa5b14e861adef01d9abd44ce82a85e19d6e21d3a4")
        ];
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId, //XXX seems to require a where clause with reference to T to compile
    {
        XXXPhantomFakeEvent(AccountId), // XXX

        /// XXX
        MagicExtract(CashAmount, AccountIdent, Notice<Ethereum>),

        /// An Ethereum event was successfully processed. [payload]
        ProcessedEthEvent(SignedPayload),

        /// An Ethereum event failed during processing. [payload, reason]
        FailedProcessingEthEvent(SignedPayload, Reason),

        /// Signed Ethereum notice. [message, signatures]
        SignedNotice(notices::Message, notices::Signatures), // XXX emit Payload? NoticePayload? should be common to all notices
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        // XXX
        /// Error names should be descriptive.
        NoneValue,

        // XXX
        /// Errors should have helpful documentation associated with them.
        StorageOverflow,

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

        // XXX this function is temporary and will be deleted after we reach a certain point
        /// An example dispatchable that takes a singles value as a parameter, writes the value to
        /// storage and emits an event. This function must be dispatched by a signed extrinsic.
        #[weight = 10_000 + T::DbWeight::get().writes(1)]
        pub fn magic_extract(origin, account: AccountIdent, amount: CashAmount) -> dispatch::DispatchResult {
            let () = ensure_none(origin)?;

            // Update storage -- TODO: increment this-- sure why not?
            let curr_cash_balance: CashAmount = CashBalance::get(&account).unwrap_or_default();
            let next_cash_balance: CashAmount = curr_cash_balance.checked_add(amount).ok_or(Error::<T>::StorageOverflow)?;
            CashBalance::insert(&account, next_cash_balance);

            let now = <frame_system::Module<T>>::block_number().saturated_into::<u8>();
            // Add to Notice Queue
            let notice = Notice::ExtractionNotice {
                id: (now.into(), 0),  // XXX need to keep state of current gen/within gen for each, also parent
                parent: [0u8; 32], // XXX,
                asset: [0u8; 20],
                amount: amount,
                account: account.address.clone().try_into() // XXX avoid clone?
                    .unwrap_or_else(|_| panic!("Address has wrong number of bytes"))
            };
            EthNoticeQueue::insert(notice.id(), NoticeStatus::<Ethereum>::Pending { signers: vec![], signatures: vec![], notice: notice.clone()});

            // Emit an event.
            // Self::deposit_event(RawEvent::MagicExtract(amount, account, notice));
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
            let recovery_id = secp256k1::RecoveryId::parse(sig[64]).map_err(|_| <Error<T>>::SignedPayloadError)?;
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

            let event = chains::eth::decode(payload.as_slice()); // XXX
            let status = <EthEventQueue>::get(event.id).unwrap_or(EventStatus::<Ethereum>::Pending { signers: vec![] }); // XXX
            match status {
                EventStatus::<Ethereum>::Pending { signers } => {
                    // XXX sets?
                    debug::native::info!("Signers {:?}", signers);
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
                           Self::deposit_event(RawEvent::SignedNotice(notices::encode_ethereum_notice(notice), signatures)); // XXX generic
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

        // XXX
        /// An example dispatchable that may throw a custom error.
        #[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
        pub fn cause_error(origin) -> dispatch::DispatchResult {
            let _who = ensure_signed(origin)?;
            let _ = runtime_interfaces::config_interface::get();
            Err(Error::<T>::NoneValue)?
        }


        /// Offchain Worker entry point.
        fn offchain_worker(block_number: T::BlockNumber) {
            debug::native::info!("Hello World from offchain workers from block {} !", block_number);

            let result = Self::fetch_events_with_lock();
            if let Err(e) = result {
                debug::native::error!("offchain_worker error: {:?}", e);
            }
            Self::process_eth_notices(block_number);
        }
    }
}

/// Reading error messages inside `decl_module!` can be difficult, so we move them here.
impl<T: Config> Module<T> {
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
        // TODO: This is not ready for prime-time
        ValidTransaction::with_tag_prefix("CashPallet")
            // The transaction is only valid for next 10 blocks. After that it's
            // going to be revalidated by the pool.
            .longevity(10)
            // .and_provides("fix_this_function") /// XXX this causes an error, disable for now
            // It's fine to propagate that transaction to other peers, which means it can be
            // created even by nodes that don't produce blocks.
            // Note that sometimes it's better to keep it for yourself (if you are the block
            // producer), since for instance in some schemes others may copy your solution and
            // claim a reward.
            .propagate(true)
            .build()
    }
}
