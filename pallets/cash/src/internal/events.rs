use codec::Encode;
use frame_support::storage::{IterableStorageMap, StorageMap};
use frame_system::offchain::SubmitTransaction;
use sp_runtime::offchain::{
    storage::StorageValueRef,
    storage_lock::{StorageLock, Time},
};

use our_std::collections::btree_set::BTreeSet;

use crate::{
    chains::{
        Chain, ChainBlock, ChainBlockNumber, ChainBlockTally, ChainBlocks, ChainEvent, ChainEvents,
        ChainHash, ChainId, ChainReorg, ChainReorgTally, ChainSignature, Ethereum,
    },
    core::{apply_chain_event_internal, recover_validator, validator_sign},
    events::{fetch_chain_block, fetch_chain_blocks, filter_chain_blocks},
    log,
    reason::Reason,
    require,
    types::ValidatorIdentity,
    Call, Config, Event as EventT, IngressionQueue, LastProcessedBlock, Module, PendingChainBlocks,
    PendingChainReorgs,
};

/// Incrementally perform the next step of tracking events from all the underlying chains.
pub fn track_chain_events<T: Config>() -> Result<(), Reason> {
    const OCW_TRACK_CHAIN_EVENTS: &[u8; 24] = b"cash::track_chain_events";
    let mut lock = StorageLock::<Time>::new(OCW_TRACK_CHAIN_EVENTS);
    let result = match lock.try_lock() {
        Ok(_guard) => {
            // Note: chains could be parallelized
            track_chain_events_on::<T>(ChainId::Eth)
        }

        _ => Err(Reason::WorkerBusy),
    };
    result
}

/// Perform the next step of tracking events from an underlying chain.
pub fn track_chain_events_on<T: Config>(chain_id: ChainId) -> Result<(), Reason> {
    let me = sp_core::crypto::AccountId32::new([0u8; 32]); // XXX self worker identity
    let last_block = LastProcessedBlock::get(chain_id).ok_or(Reason::NoSuchBlock)?;
    let true_block = fetch_chain_block(chain_id, last_block.number())?;
    if last_block.hash() == true_block.hash() {
        // worker sees same fork as chain
        let pending_blocks = PendingChainBlocks::get(chain_id);
        let event_queue = IngressionQueue::get(chain_id).ok_or(Reason::NoSuchQueue)?;
        let slack = calculate_queue_slack::<T>(chain_id, &event_queue);
        let blocks = fetch_chain_blocks(chain_id, last_block.number(), slack)?;
        let filtered = filter_chain_blocks(chain_id, pending_blocks, blocks);
        submit_chain_blocks::<T>(chain_id, filtered)
    } else {
        // worker sees a different fork
        let reorg = formulate_reorg(chain_id, last_block, true_block)?;
        if !already_voted_for_reorg::<T>(chain_id, me, &reorg) {
            submit_chain_reorg::<T>(chain_id, &reorg)
        } else {
            // just wait for the reorg to succeed or fail,
            //  or we change our minds in another pass (noop)
            Ok(())
        }
    }
}

/// Determine the number of blocks which can still fit on the ingression queue.
pub fn calculate_queue_slack<T: Config>(chain_id: ChainId, _event_queue: &ChainEvents) -> u32 {
    1 // XXX look at ingression queue, how backed up is the queue?
}

// XXX and move to src/events?
pub fn risk_adjusted_value(
    chain_event: &ChainEvent,
    block_number: ChainBlockNumber,
) -> crate::types::USDQuantity {
    // XXX
    // XXX
    //  if Lock or LockCash -> Lock amount value risk decayed by blocks elapsed since block number
    //  Gov -> max/set value probably also decayed since block number
    //  otherwise -> 0, trx requests, what else?
    // XXX actually abstract out (type, amount) from each and do universally?
    // match chain_event {

    // }
    crate::types::Quantity::from_nominal("1000", crate::types::USD) // XXX
}

// XXX
// pub fn extract_batch(&mut self, quota: USDQuantity) {
//     // XXXX
//     let available = QUOTA;

// }

/// Ingress a single round (quota per underlying chain block ingested).
pub fn ingress_queue<T: Config>(
    chain_id: ChainId,
    last_block: &ChainBlock,
    event_queue: &mut ChainEvents,
) -> Result<(), Reason> {
    use crate::types::{Quantity, USDQuantity, USD};
    const QUOTA: USDQuantity = Quantity::from_nominal("10000", USD); // XXX value? per chain?
    let available = QUOTA;
    //XXX let batch = extract_batch(QUOTA, &event_queue);
    // for event in batch {
    //  match apply ... err -> emit
    for event in event_queue {
        let value = risk_adjusted_value(&event, last_block.number());
        if value < available {
            // XXX available = available.sub(value);
            match apply_chain_event_internal::<T>(event) {
                Ok(()) => {
                    // XXX remove from queue
                }

                Err(reason) => {
                    // XXX emit event?
                    // XXX remove and dont reprocess, right?
                }
            }
        }
    }

    Ok(()) // XXX
}

// XXX whatever we submit we must store first so we can formulate reorgs
//  worry about pruning history independently / later
//  but for both blocks added in chain_blocks or reorgs

/// Submit the underlying chain blocks the worker calculates are needed by the chain next.
pub fn submit_chain_blocks<T: Config>(
    chain_id: ChainId,
    blocks: ChainBlocks,
) -> Result<(), Reason> {
    log!("Submitting chain blocks extrinsic: {:?}", blocks);

    // XXX why are we signing with eth?
    //  bc eth is identity key...
    let signature = validator_sign::<T>(&blocks.encode()[..])?;
    let call = Call::receive_chain_blocks(blocks, signature);

    // TODO: Do we want to short-circuit on an error here?
    let res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
        .map_err(|()| Reason::FailedToSubmitExtrinsic);

    if res.is_err() {
        log!("Error while sending event extrinsic");
    }

    Ok(())
}

// XXX move to src/events?
/// Try to form a path from the last block to the new true block.
pub fn formulate_reorg(
    chain_id: ChainId,
    last_block: ChainBlock,
    true_block: ChainBlock,
) -> Result<ChainReorg, Reason> {
    // XXX try to form a path from last to truth
    // XXX just always N == K?
    const N: u32 = 10;
    const K: u32 = 10;
    let mut reverse_blocks: Vec<ChainBlock> = vec![]; // XXX convert to raw_block for type
    let mut drawrof_blocks: Vec<ChainBlock> = vec![]; // XXX convert to raw_block and reverse
    let mut reverse_hashes: BTreeSet<ChainHash>; // XXX init empty?
    loop {
        // let reverse_blocks_next = storage.walk_backwards(last_block.hash(), N)?;
        // let drawrof_blocks_next = storage.walk_backwards(truth.hash(), K)?
        // reverse_blocks.extend_from_slice(reverse_blocks_next)
        // reverse_hashes.extend index with reverse_blocks_next
        // for block in drawrof_blocks_next {
        //    if block.parent_hash in reverse_hashes {
        //     reverse_blocks = take until block.parent_hash (stop before common ancestor)
        //     drawrof_blocks = take until block.parent_hash
        break;
        //    } else {
        //     drawrof_blocks.push(block);
        //    }
    }

    match (last_block.hash(), true_block.hash()) {
        (ChainHash::Eth(from_hash), ChainHash::Eth(to_hash)) => Ok(ChainReorg::Eth {
            from_hash,
            to_hash,
            reverse_blocks: vec![], // XXX convert reverse_blocks to raw blocks for chain
            forward_blocks: vec![], // XXX convert drawrof_blocks to raw blocks and reverse
        }),

        _ => panic!("xxx not implemented"),
    }
}

// XXX & refs here not blocks?
fn already_voted_for_reorg<T: Config>(
    chain_id: ChainId,
    id: ValidatorIdentity,
    reorg: &ChainReorg,
) -> bool {
    // XXX dont revote for same thing
    false
}

fn submit_chain_reorg<T: Config>(chain_id: ChainId, reorg: &ChainReorg) -> Result<(), Reason> {
    log!("Submitting chain reorg extrinsic: {:?}", reorg);
    // XXX
    Ok(())
}

pub fn receive_chain_blocks<T: Config>(
    blocks: ChainBlocks,
    signature: ChainSignature,
) -> Result<(), Reason> {
    let validator = recover_validator::<T>(&blocks.encode(), signature)?;
    let chain_id = blocks.chain_id();
    let mut last_block = LastProcessedBlock::get(chain_id).ok_or(Reason::NoSuchBlock)?;
    let mut pending_blocks = PendingChainBlocks::get(chain_id);
    let mut event_queue = IngressionQueue::get(chain_id).ok_or(Reason::NoSuchQueue)?;
    for block in blocks {
        if block.number() >= last_block.number() + 1 {
            let offset = (block.number() - last_block.number() - 1) as usize;
            if let Some(prior) = pending_blocks.get(offset) {
                if block != prior.block {
                    // XXX actually, vote against?
                    continue;
                }
                // XXX best way to store these?
                pending_blocks[offset] = prior.clone().with_signer(validator.substrate_id.clone());
            // XXX
            } else if offset == 0 {
                if block.parent_hash() != last_block.hash() {
                    // worker should reorg if it wants to build something else
                    //  but could just be a laggard message
                    continue;
                }
                pending_blocks[offset] =
                    ChainBlockTally::new(chain_id, block, validator.substrate_id.clone());
            } else if let Some(parent) = pending_blocks.get(offset - 1) {
                if block.parent_hash() != parent.block.hash() {
                    // worker is submitting block for parent that doesnt exist
                    //  if its also submitting the parent, which it should be,
                    //   that one will vote against and propagate forwards
                    continue;
                }
                pending_blocks[offset] =
                    ChainBlockTally::new(chain_id, block, validator.substrate_id.clone());
            }
        }
    }

    for tally in pending_blocks.clone().iter() {
        if tally.clone().passes_threshold() {
            //XXX
            // XXX 'forward block'?
            event_queue.push(&tally.block);
            last_block = tally.block.clone(); // XXX
            pending_blocks.remove(0); // XXX assert 0 == tally?
            ingress_queue::<T>(chain_id, &last_block, &mut event_queue)?;
        } else {
            break;
        }
    }

    LastProcessedBlock::insert(chain_id, last_block);
    PendingChainBlocks::insert(chain_id, pending_blocks);
    IngressionQueue::insert(chain_id, event_queue);

    Ok(())
}

pub fn receive_chain_reorg<T: Config>(
    reorg: ChainReorg,
    signature: ChainSignature,
) -> Result<(), Reason> {
    let validator = recover_validator::<T>(&reorg.encode(), signature)?;
    let chain_id = reorg.chain_id();
    let mut event_queue = IngressionQueue::get(chain_id).ok_or(Reason::NoSuchQueue)?;
    let mut last_block = LastProcessedBlock::get(chain_id).ok_or(Reason::NoSuchBlock)?;
    let mut pending_blocks = PendingChainReorgs::get(chain_id);

    // Note: Can reject / stop propagating once this check fails
    require!(reorg.from_hash() == last_block.hash(), Reason::HashMismatch);

    // XXX how to mutate pending?
    let tally = if let Some(prior) = pending_blocks.iter().find(|&r| r.reorg == reorg) {
        // XXXX some sort of mutate in place
        //  stops including after threshold is passed?
        //   rather just check passes threshold twice (if passed_threshold and tally.passes_threshold()...)
        prior.clone().with_signer(validator.substrate_id) // XXX
    } else {
        ChainReorgTally::new(chain_id, reorg, validator.substrate_id)
    };

    // XXX specifically it should pass now but not previously before
    //  should with_signature stop including once done?
    //   tricky bc we need to add sigs then check in order after for completion
    if tally.clone().passes_threshold() {
        // XXX
        // XXX perform reorg
        //  reverse blocks
        //   undo events given (if lock -> debit account, interest is lost to system)
        //    they earn a bit of others interest, but total interest is same
        //  forward blocks
        //   push/ingress ev queue, update last block/pending
        //  pass last block for them to modify
        //   returns last block
        LastProcessedBlock::insert(chain_id, last_block);
        PendingChainBlocks::insert(chain_id, Vec::<ChainBlockTally>::new());
        PendingChainReorgs::insert(chain_id, Vec::<ChainReorgTally>::new());
        IngressionQueue::insert(chain_id, event_queue);
    } else {
        PendingChainReorgs::insert(chain_id, pending_blocks);
    }

    Ok(())
}

// XXX delete
// pub fn receive_event<T: Config>(
//     event_id: ChainLogId,
//     event: ChainLogEvent,
//     signature: ChainSignature,
// ) -> Result<(), Reason> {
//     log!(
//         "receive_event({:?}, {:?}, {})",
//         &event_id,
//         &event,
//         hex::encode(signature)
//     );

//     let signer = recover_validator::<T>(&reorg.encode(), signature)?;

//     match EventStates::get(event_id) {
//         EventState::Pending { signers } => {
//             // Add new validator to the signers
//             let mut signers_new = signers.clone();
//             signers_new.insert(signer);

//             if passes_validation_threshold(&signers_new, &validators) {
//                 match apply_chain_event_internal::<T>(event) {
//                     Ok(()) => {
//                         EventStates::insert(event_id, EventState::Done);
//                         <Module<T>>::deposit_event(EventT::ProcessedChainEvent(event_id));
//                         Ok(())
//                     }

//                     Err(reason) => {
//                         log!(
//                             "receive_event_internal({}) apply failed: {:?}",
//                             event_id.show(),
//                             reason
//                         );
//                         EventStates::insert(event_id, EventState::Failed { reason });
//                         <Module<T>>::deposit_event(EventT::FailedProcessingChainEvent(
//                             event_id, reason,
//                         ));
//                         Ok(())
//                     }
//                 }
//             } else {
//                 log!(
//                     "receive_event_internal({}) signer_count={}",
//                     event_id.show(),
//                     signers_new.len()
//                 );
//                 EventStates::insert(
//                     event_id,
//                     EventState::Pending {
//                         signers: signers_new,
//                     },
//                 );
//                 Ok(())
//             }
//         }

//         EventState::Failed { .. } | EventState::Done => {
//             // TODO: Eventually we should be deleting or retrying here (based on monotonic ids and signing order)
//             Ok(())
//         }
//     }
// }
