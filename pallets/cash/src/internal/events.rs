use codec::Encode;
use frame_support::storage::{IterableStorageMap, StorageDoubleMap};
use frame_system::offchain::SubmitTransaction;
use our_std::collections::btree_set::BTreeSet;
use sp_runtime::offchain::{
    storage::StorageValueRef,
    storage_lock::{StorageLock, Time},
};

use crate::{
    chains::{Chain, ChainId, Ethereum},
    core::{apply_chain_event_internal, passes_validation_threshold},
    events::{encode_block_hex, fetch_eth_events, ChainLogEvent, ChainLogId},
    log,
    reason::Reason,
    types::{SignersSet, ValidatorSig},
    Call, Config, DoneEvents, Event as EventT, FailedEvents, Module, PendingEvents, Validators,
};

// OCW storage constants
const OCW_STORAGE_LOCK_ETHEREUM_EVENTS: &[u8; 34] = b"cash::storage_lock_ethereum_events";
const OCW_LATEST_CACHED_ETHEREUM_BLOCK: &[u8; 34] = b"cash::latest_cached_ethereum_block";

// XXX disambiguate whats for ethereum vs not
pub fn process_events<T: Config>() -> Result<(), Reason> {
    let s_info = StorageValueRef::persistent(OCW_LATEST_CACHED_ETHEREUM_BLOCK);

    let from_block: String = if let Some(Some(cached_block_num)) = s_info.get::<u64>() {
        // Ethereum block number has been cached, fetch events starting from the next after cached block
        log!("Last cached block number: {:?}", cached_block_num);
        encode_block_hex(cached_block_num + 1)
    } else {
        // Validator's cache is empty, fetch events from the earliest block with pending events
        log!("Block number has not been cached yet");
        // XXX TODO: Add back?
        // let block_numbers: Vec<u32> = EthEventQueue::iter()
        //     .filter_map(|((block_number, _log_index), status)| {
        //         if match status {
        //             EventStatus::<Ethereum>::Pending { .. } => true,
        //             _ => false,
        //         } {
        //             Some(block_number)
        //         } else {
        //             None
        //         }
        //     })
        //     .collect();
        // let pending_events_block = block_numbers.iter().min();
        // if pending_events_block.is_some() {
        //     let events_block: u32 = *pending_events_block.unwrap();
        //     from_block = format!("{:#X}", events_block);
        // } else {
        //}
        String::from("earliest")
    };

    log!("Fetching events starting from block {:?}", from_block);

    let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK_ETHEREUM_EVENTS);
    if let Ok(_guard) = lock.try_lock() {
        match fetch_eth_events(from_block) {
            Ok(event_info) => {
                log!("Result: {:?}", event_info);

                // TODO: The failability here is bad, since we don't want to re-process events
                // We need to make sure this is fully idempotent

                // Send extrinsics for all events
                submit_events::<T>(ChainId::Eth, event_info.events)?;

                // Save latest block in ocw storage
                s_info.set(&event_info.latest_eth_block);
            }
            Err(err) => {
                log!("Error while fetching events: {:?}", err);
                return Err(Reason::FetchError);
            }
        }
    }
    Ok(())
}

fn submit_events<T: Config>(
    chain_id: ChainId,
    events: Vec<(ChainLogId, ChainLogEvent)>,
) -> Result<(), Reason> {
    for (event_id, event) in events.into_iter() {
        log!(
            "Processing event and sending extrinsic: {} {:?}",
            event_id.show(),
            event
        );

        // XXX why are we signing with eth?
        //  bc eth is identity key...
        let signature = <Ethereum as Chain>::sign_message(&event.encode()[..])?;
        let call = Call::receive_event(chain_id, event_id, event, signature);

        // TODO: Do we want to short-circuit on an error here?
        let res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
            .map_err(|()| Reason::FailedToSubmitExtrinsic);

        if res.is_err() {
            log!("Error while sending event extrinsic");
        }
    }
    Ok(())
}

pub fn receive_event<T: Config>(
    chain_id: ChainId,
    event_id: ChainLogId,
    event: ChainLogEvent,
    signature: ValidatorSig,
) -> Result<(), Reason> {
    // XXX sig
    // XXX do we want to store/check hash to allow replaying?
    // TODO: use more generic function?
    // XXX why is this using eth for validator sig though?
    let signer: crate::types::ValidatorIdentity =
        compound_crypto::eth_recover(&event.encode()[..], &signature, false)?;
    let validators: BTreeSet<_> = Validators::iter().map(|v| v.1.eth_address).collect();
    if !validators.contains(&signer) {
        log!(
            "Signer of a log event is not a known validator {:?}, validators are {:?}",
            signer,
            validators
        );
        return Err(Reason::UnknownValidator)?;
    }

    // Check if event is in `Done` or `Failed` queues first,
    // Otherwise it is a new or already seen `Pending` event
    if DoneEvents::contains_key(chain_id, event_id)
        || FailedEvents::contains_key(chain_id, event_id)
    {
        // TODO: Eventually we should be deleting or retrying here (based on monotonic ids and signing order)
        return Ok(());
    }

    // Get signers set, if it's a new `Pending` event, create a new empty signers set
    let mut signers = if PendingEvents::contains_key(chain_id, event_id) {
        PendingEvents::take(chain_id, event_id)
    } else {
        SignersSet::new()
    };
    signers.insert(signer);

    if passes_validation_threshold(&signers, &validators) {
        match apply_chain_event_internal::<T>(event) {
            Ok(()) => {
                DoneEvents::insert(chain_id, event_id, signers);
                <Module<T>>::deposit_event(EventT::ProcessedChainEvent(chain_id, event_id));
                Ok(())
            }

            Err(reason) => {
                log!(
                    "process_chain_event_internal({}) apply failed: {:?}",
                    event_id.show(),
                    reason
                );
                FailedEvents::insert(chain_id, event_id, reason);
                <Module<T>>::deposit_event(EventT::FailedProcessingChainEvent(
                    chain_id, event_id, reason,
                ));
                Ok(())
            }
        }
    } else {
        log!(
            "process_chain_event_internal({}) signer_count={}",
            event_id.show(),
            signers.len()
        );
        PendingEvents::insert(chain_id, event_id, signers);
        Ok(())
    }
}
