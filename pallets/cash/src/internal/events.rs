use codec::Encode;
use frame_support::storage::{IterableStorageMap, StorageMap};
use frame_system::offchain::SubmitTransaction;
use our_std::collections::btree_set::BTreeSet;
use sp_runtime::offchain::{
    storage::StorageValueRef,
    storage_lock::{StorageLock, Time},
};

use crate::{
    chains::{
        Chain, ChainBlock, ChainBlockNumber, ChainBlockTally, ChainBlocks, ChainHash, ChainId,
        ChainReorg, ChainReorgTally, Ethereum,
    },
    core::{apply_chain_event_internal, passes_validation_threshold},
    events::{encode_block_hex, fetch_eth_blocks, ChainLogEvent, ChainLogId, EventState},
    log,
    reason::Reason,
    require,
    types::{ValidatorIdentity, ValidatorSig},
    Call, Config, Event as EventT, EventStates, IngressionQueue, LastProcessedBlock, Module,
    PendingChainBlocks, PendingChainReorgs, Validators,
};

// OCW storage constants
const OCW_STORAGE_LOCK_ETH_EVENTS: &[u8; 29] = b"cash::storage_lock_eth_events";
const OCW_LATEST_CACHED_ETH_BLOCK: &[u8; 29] = b"cash::latest_cached_eth_block";

pub fn track_chain_events<T: Config>() -> Result<(), Reason> {
    // XXX do we need to lock ocw?
    // Note: these chains can be parallelized
    track_eth_events::<T>()
}

pub fn track_eth_events<T: Config>() -> Result<(), Reason> {
    let me = [0u8; 20]; // XXX self worker identity
    let chain_id = ChainId::Eth;
    let last = LastProcessedBlock::get(chain_id).ok_or(Reason::NoSuchBlock)?;
    let truth = fetch_chain_block(chain_id, last.number())?;
    if last.hash() == truth.hash() {
        let slack = calculate_queue_slack::<T>(chain_id);
        if slack >= 0 {
            let pending = PendingChainBlocks::get(chain_id);
            let blocks = fetch_chain_blocks(chain_id, last.number(), slack)?;
            let filtered = filter_chain_blocks(chain_id, pending, blocks);
            submit_chain_blocks::<T>(chain_id, filtered)
        } else {
            // no slack
            // just let the Gateway chain catch up (noop)
            Ok(())
        }
    } else {
        // different fork
        let reorg = formulate_reorg(chain_id, last, truth)?;
        if !already_voted_for_reorg(chain_id, me, reorg) {
            submit_chain_reorg::<T>(chain_id, reorg)
        } else {
            // just wait for the reorg to succeed or fail,
            //  or we change our minds in another pass (noop)
            Ok(())
        }
    }
}

fn calculate_queue_slack<T: Config>(chain_id: ChainId) -> u32 {
    0 // XXX look at ingression queue, how backed up?
}

pub fn fetch_chain_block(
    chain_id: ChainId,
    number: ChainBlockNumber,
) -> Result<ChainBlock, Reason> {
    // XXX
    let from_block: String = encode_block_hex(number);

    let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK_ETH_EVENTS);
    if let Ok(_guard) = lock.try_lock() {
        match fetch_eth_blocks(from_block) {
            Ok(event_info) => {
                log!("Result: {:?}", event_info);

                // XXXXXXX submitted elsewhere now, this *just* gets the block
                //  do we need caching block number at all now?

                // TODO: The failability here is bad, since we don't want to re-process events
                // We need to make sure this is fully idempotent

                // Send extrinsics for all events
                // XXX submit_blocks::<T>(event_info.events)
                Ok(ChainBlock::Eth(
                    crate::chains::eth::Block {
                        hash: [1u8; 32],
                        parent_hash: [0u8; 32],
                        number: 1,
                        events: vec![],
                    })) // XXX
            }

            Err(err) => {
                log!("Error while fetching events: {:?}", err);
                Err(Reason::FetchError)
            }
        }
    } else {
        Err(Reason::WorkerFailure)
    }
}

fn fetch_chain_blocks(
    chain_id: ChainId,
    number: ChainBlockNumber,
    slack: u32,
) -> Result<ChainBlocks, Reason> {
    // XXX fetch given slack
    Ok(ChainBlocks::Eth(vec![])) // XXX eth
}

fn filter_chain_blocks(
    chain_id: ChainId,
    pending: Vec<ChainBlockTally>,
    blocks: ChainBlocks,
) -> ChainBlocks {
    // XXX fetch given pending queue (eliminate vote?)
    //  what can we really eliminate since we need negative vote?
    blocks
}

fn submit_chain_blocks<T: Config>(chain_id: ChainId, blocks: ChainBlocks) -> Result<(), Reason> {
    log!("Submitting chain blocks extrinsic: {:?}", blocks);

    // XXX why are we signing with eth?
    //  bc eth is identity key...
    let signature = <Ethereum as Chain>::sign_message(&blocks.encode()[..])?;
    let call = Call::receive_chain_blocks(blocks, signature);

    // TODO: Do we want to short-circuit on an error here?
    let res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
        .map_err(|()| Reason::FailedToSubmitExtrinsic);

    if res.is_err() {
        log!("Error while sending event extrinsic");
    }

    Ok(())
}

fn formulate_reorg(
    chain_id: ChainId,
    last: ChainBlock,
    truth: ChainBlock,
) -> Result<ChainReorg, Reason> {
    // XXX try to form a path from last to truth
    //  how do we actually form a reverse path without storing it?
    //   (workers *must* store it)
    match (last.hash(), truth.hash()) {
        (ChainHash::Eth(from_hash), ChainHash::Eth(to_hash)) => Ok(ChainReorg::Eth {
            from_hash,
            to_hash,
            reverse_blocks: vec![], // XXX
            forward_blocks: vec![], // XXX
        }),
    }
}

fn already_voted_for_reorg(chain_id: ChainId, id: ValidatorIdentity, reorg: ChainReorg) -> bool {
    // XXX dont revote for same thing
    false
}

fn submit_chain_reorg<T: Config>(chain_id: ChainId, reorg: ChainReorg) -> Result<(), Reason> {
    log!("Submitting chain reorg extrinsic: {:?}", reorg);

    Ok(())
}

// XXX
pub fn recover_validator<T: Config>(
    data: &[u8],
    signature: ValidatorSig,
) -> Result<ValidatorIdentity, Reason> {
    let signer = <Ethereum as Chain>::recover_address(data, signature)?;
    let validators: BTreeSet<_> = Validators::iter().map(|v| v.1.eth_address).collect();
    if !validators.contains(&signer) {
        return Err(Reason::UnknownValidator);
    }
    Ok(signer)
}

pub fn receive_chain_blocks<T: Config>(
    blocks: ChainBlocks,
    signature: ValidatorSig,
) -> Result<(), Reason> {
    let validator = recover_validator::<T>(&blocks.encode(), signature)?;
    let chain_id = blocks.chain_id();
    let ev_queue = IngressionQueue::get(chain_id).ok_or(Reason::NoSuchQueue)?;
    let mut last = LastProcessedBlock::get(chain_id).ok_or(Reason::NoSuchBlock)?;
    let mut pending = PendingChainBlocks::get(chain_id);
    for block in blocks {
        if block.number() >= last.number() + 1 {
            let offset = block.number() - last.number() - 1;
            if let Some(prior) = pending.get(offset) {
                if block != prior.block {
                    // XXX actually, vote against?
                    continue;
                }
                // XXX best way to store these?
                pending[offset] = prior.with_signature(validator);
            } else if offset == 0 {
                if block.parent_hash() != last.hash() {
                    // worker should reorg if it wants to build something else
                    //  but could just be a laggard message
                    continue;
                }
                pending[offset] = ChainBlockTally::new(chain_id, block, validator);
            } else if let Some(parent) = pending.get(offset - 1) {
                if block.parent_hash() != parent.hash() {
                    // worker is submitting block for parent that doesnt exist
                    //  if its also submitting the parent, which it should be,
                    //   that one will vote against and propagate forwards
                    continue;
                }
                pending[offset] = ChainBlockTally::new(chain_id, block, validator);
            }
        }
    }

    for tally in pending.iter() {
        if tally.passes_threshold() {
            ev_queue.push(tally.block);
            last = tally.block;
            pending.remove(0); // XXX assert?
        } else {
            break;
        }
    }

    IngressionQueue::insert(chain_id, ev_queue);
    LastProcessedBlock::insert(chain_id, last);
    PendingChainBlocks::insert(chain_id, pending);

    Ok(())
}

pub fn receive_chain_reorg<T: Config>(
    reorg: ChainReorg,
    signature: ValidatorSig,
) -> Result<(), Reason> {
    let validator = recover_validator::<T>(&reorg.encode(), signature)?;
    let chain_id = reorg.chain_id();
    let ev_queue = IngressionQueue::get(chain_id).ok_or(Reason::NoSuchQueue)?;
    let mut last = LastProcessedBlock::get(chain_id).ok_or(Reason::NoSuchBlock)?;
    let mut pending = PendingChainReorgs::get(chain_id);

    // Note: Can reject / stop propagating once this check fails
    require!(reorg.from_hash() == last.hash(), Reason::HashMismatch);

    // XXX how to mutate pending?
    let mut tally = if let Some(prior) = pending.iter().find(|&&r| r.reorg == reorg) {
        // XXXX some sort of mutate in place
        //  stops including after threshold is passed?
        //   rather just check passes threshold twice (if passed_threshold and tally.passes_threshold()...)
        prior.with_signature(validator);
        prior
    } else {
        ChainReorgTally::new(chain_id, reorg, validator)
    };

    // XXX specifically passes now but didnt before
    //  should with_signature stop including once done?
    if tally.passes_threshold() {
        // XXX perform reorg
        //  reverse blocks
        //  forward blocks
        //  pass last block for them to modify
        //   returns last block
        IngressionQueue::insert(chain_id, ev_queue);
        LastProcessedBlock::insert(chain_id, last);
        PendingChainBlocks::insert(chain_id, vec![]);
        PendingChainReorgs::insert(chain_id, vec![]);
    } else {
        PendingChainReorgs::insert(chain_id, pending);
    }

    Ok(())
}

// XXX delete
pub fn receive_event<T: Config>(
    event_id: ChainLogId,
    event: ChainLogEvent,
    signature: ValidatorSig,
) -> Result<(), Reason> {
    log!(
        "receive_event({:?}, {:?}, {})",
        &event_id,
        &event,
        hex::encode(signature)
    );

    // XXX sig
    // XXX do we want to store/check hash to allow replaying?
    // TODO: use more generic function?
    // XXX why is this using eth for validator sig though?
    let signer = <Ethereum as Chain>::recover_address(&event.encode(), signature)?;
    let validators: BTreeSet<_> = Validators::iter().map(|v| v.1.eth_address).collect();
    if !validators.contains(&signer) {
        log!(
            "Signer of a log event is not a known validator {:?}, validators are {:?}",
            signer,
            validators
        );
        return Err(Reason::UnknownValidator)?;
    }

    match EventStates::get(event_id) {
        EventState::Pending { signers } => {
            // Add new validator to the signers
            let mut signers_new = signers.clone();
            signers_new.insert(signer);

            if passes_validation_threshold(&signers_new, &validators) {
                match apply_chain_event_internal::<T>(event) {
                    Ok(()) => {
                        EventStates::insert(event_id, EventState::Done);
                        <Module<T>>::deposit_event(EventT::ProcessedChainEvent(event_id));
                        Ok(())
                    }

                    Err(reason) => {
                        log!(
                            "receive_event_internal({}) apply failed: {:?}",
                            event_id.show(),
                            reason
                        );
                        EventStates::insert(event_id, EventState::Failed { reason });
                        <Module<T>>::deposit_event(EventT::FailedProcessingChainEvent(
                            event_id, reason,
                        ));
                        Ok(())
                    }
                }
            } else {
                log!(
                    "receive_event_internal({}) signer_count={}",
                    event_id.show(),
                    signers_new.len()
                );
                EventStates::insert(
                    event_id,
                    EventState::Pending {
                        signers: signers_new,
                    },
                );
                Ok(())
            }
        }

        EventState::Failed { .. } | EventState::Done => {
            // TODO: Eventually we should be deleting or retrying here (based on monotonic ids and signing order)
            Ok(())
        }
    }
}
