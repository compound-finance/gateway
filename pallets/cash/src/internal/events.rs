use codec::Encode;
use frame_support::storage::{IterableStorageMap, StorageMap};
use frame_system::offchain::SubmitTransaction;
use sp_runtime::offchain::{
    storage::StorageValueRef,
    storage_lock::{StorageLock, Time},
};

use crate::{
    chains::{
        Chain, ChainBlock, ChainBlockNumber, ChainBlockTally, ChainBlocks, ChainEvents, ChainHash,
        ChainId, ChainReorg, ChainReorgTally, ChainSignature, Ethereum,
    },
    core::{apply_chain_event_internal, recover_validator, validator_sign},
    events::{encode_block_hex, fetch_eth_blocks, ChainLogEvent, ChainLogId, EventState},
    log,
    reason::Reason,
    require,
    types::ValidatorIdentity,
    Call, Config, Event as EventT, EventStates, IngressionQueue, LastProcessedBlock, Module,
    PendingChainBlocks, PendingChainReorgs,
};

pub fn track_chain_events<T: Config>() -> Result<(), Reason> {
    const OCW_TRACK_CHAIN_EVENTS: &[u8; 24] = b"cash::track_chain_events";
    let mut lock = StorageLock::<Time>::new(OCW_TRACK_CHAIN_EVENTS);
    let result = match lock.try_lock() {
        Ok(_guard) => {
            // Note: chains could be parallelized
            track_eth_events::<T>()
        }

        _ => Err(Reason::WorkerBusy)
    };
    result
}

pub fn track_eth_events<T: Config>() -> Result<(), Reason> {
    let me = sp_core::crypto::AccountId32::new([0u8; 32]); // XXX self worker identity
    let chain_id = ChainId::Eth;
    let last = LastProcessedBlock::get(chain_id).ok_or(Reason::NoSuchBlock)?;
    let truth = fetch_chain_block(chain_id, last.number())?;
    if last.hash() == truth.hash() {
        // worker sees same fork as chain
        let slack = calculate_queue_slack::<T>(chain_id);
        let pending = PendingChainBlocks::get(chain_id);
        let blocks = fetch_chain_blocks(chain_id, last.number(), slack)?;
        let filtered = filter_chain_blocks(chain_id, pending, blocks);
        submit_chain_blocks::<T>(chain_id, filtered)
    } else {
        // worker sees a different fork
        let reorg = formulate_reorg(chain_id, last, truth)?;
        if !already_voted_for_reorg(chain_id, me, &reorg) {
            submit_chain_reorg::<T>(chain_id, &reorg)
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

fn risk_adjusted_value(chain_event: &crate::events::ChainLogEvent, block_number: ChainBlockNumber) -> crate::types::USDQuantity { // XXX
    // XXX
    //  if Lock or LockCash -> Lock amount value risk decayed by blocks elapsed since block number
    //  Gov -> max/set value probably also decayed since block number
    //  otherwise -> 0, trx requests, what else?
    crate::types::Quantity::from_nominal("1000", crate::types::USD) // XXX
}

// Ingress a single round (quota per underlying chain block ingested).
fn ingress_queue<T: Config>(chain_id: ChainId, last_block: &ChainBlock, event_queue: &mut ChainEvents) {
    use crate::types::{Quantity, USD, USDQuantity};
    const QUOTA: USDQuantity = Quantity::from_nominal("10000", USD); // XXX per chain?
    let available = QUOTA;
    for event in event_queue.clone() { // XXX no clone
        if risk_adjusted_value(&event, last_block.number()) < available {
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
}

fn fetch_chain_block(chain_id: ChainId, number: ChainBlockNumber) -> Result<ChainBlock, Reason> {
    // XXX
    let from_block: String = encode_block_hex(number);

    match fetch_eth_blocks(from_block) {
        Ok(event_info) => {
            log!("Result: {:?}", event_info);

            // XXXXXXX submitted elsewhere now, this *just* gets the block
            //  do we need caching block number at all now?

            // TODO: The failability here is bad, since we don't want to re-process events
            // We need to make sure this is fully idempotent

            // Send extrinsics for all events
            // XXX submit_blocks::<T>(event_info.events)
            Ok(ChainBlock::Eth(crate::chains::eth::Block {
                hash: [1u8; 32],
                parent_hash: [0u8; 32],
                number: 1,
                events: vec![],
            })) // XXX
        }

        Err(err) => {
            log!("Error while fetching events: {:?}", err);
            Err(Reason::WorkerFetchError)
        }
    }
}

fn fetch_chain_blocks(
    chain_id: ChainId,
    number: ChainBlockNumber,
    slack: u32,
) -> Result<ChainBlocks, Reason> {
    // XXX fetch given slack
    //  slack 0 -> no results
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

        _ => panic!("xxx not implemented"),
    }
}

// XXX & refs here not blocks?
fn already_voted_for_reorg(chain_id: ChainId, id: ValidatorIdentity, reorg: &ChainReorg) -> bool {
    // XXX dont revote for same thing
    false
}

fn submit_chain_reorg<T: Config>(chain_id: ChainId, reorg: &ChainReorg) -> Result<(), Reason> {
    log!("Submitting chain reorg extrinsic: {:?}", reorg);

    Ok(())
}

pub fn receive_chain_blocks<T: Config>(
    blocks: ChainBlocks,
    signature: ChainSignature,
) -> Result<(), Reason> {
    let validator = recover_validator::<T>(&blocks.encode(), signature)?;
    let chain_id = blocks.chain_id();
    let mut event_queue = IngressionQueue::get(chain_id).ok_or(Reason::NoSuchQueue)?;
    let mut last = LastProcessedBlock::get(chain_id).ok_or(Reason::NoSuchBlock)?;
    let mut pending = PendingChainBlocks::get(chain_id);
    for block in blocks {
        if block.number() >= last.number() + 1 {
            let offset = (block.number() - last.number() - 1) as usize;
            if let Some(prior) = pending.get(offset) {
                if block != prior.block {
                    // XXX actually, vote against?
                    continue;
                }
                // XXX best way to store these?
                pending[offset] = prior.clone().with_signer(validator.substrate_id.clone());
            // XXX
            } else if offset == 0 {
                if block.parent_hash() != last.hash() {
                    // worker should reorg if it wants to build something else
                    //  but could just be a laggard message
                    continue;
                }
                pending[offset] =
                    ChainBlockTally::new(chain_id, block, validator.substrate_id.clone());
            } else if let Some(parent) = pending.get(offset - 1) {
                if block.parent_hash() != parent.block.hash() {
                    // worker is submitting block for parent that doesnt exist
                    //  if its also submitting the parent, which it should be,
                    //   that one will vote against and propagate forwards
                    continue;
                }
                pending[offset] =
                    ChainBlockTally::new(chain_id, block, validator.substrate_id.clone());
            }
        }
    }

    for tally in pending.clone().iter() {
        if tally.clone().passes_threshold() {
            //XXX
            // XXX 'forward block'?
            event_queue.push(&tally.block);
            last = tally.block.clone(); // XXX
            pending.remove(0); // XXX assert 0 == tally?
            ingress_queue::<T>(chain_id, &last, &mut event_queue);
        } else {
            break;
        }
    }

    IngressionQueue::insert(chain_id, event_queue);
    LastProcessedBlock::insert(chain_id, last);
    PendingChainBlocks::insert(chain_id, pending);

    Ok(())
}

pub fn receive_chain_reorg<T: Config>(
    reorg: ChainReorg,
    signature: ChainSignature,
) -> Result<(), Reason> {
    let validator = recover_validator::<T>(&reorg.encode(), signature)?;
    let chain_id = reorg.chain_id();
    let mut event_queue = IngressionQueue::get(chain_id).ok_or(Reason::NoSuchQueue)?;
    let mut last = LastProcessedBlock::get(chain_id).ok_or(Reason::NoSuchBlock)?;
    let mut pending = PendingChainReorgs::get(chain_id);

    // Note: Can reject / stop propagating once this check fails
    require!(reorg.from_hash() == last.hash(), Reason::HashMismatch);

    // XXX how to mutate pending?
    let tally = if let Some(prior) = pending.iter().find(|&r| r.reorg == reorg) {
        // XXXX some sort of mutate in place
        //  stops including after threshold is passed?
        //   rather just check passes threshold twice (if passed_threshold and tally.passes_threshold()...)
        prior.clone().with_signer(validator.substrate_id) // XXX
    } else {
        ChainReorgTally::new(chain_id, reorg, validator.substrate_id)
    };

    // XXX specifically passes now but didnt before
    //  should with_signature stop including once done?
    if tally.clone().passes_threshold() {
        // XXX
        // XXX perform reorg
        //  reverse blocks
        //  forward blocks
        //   push/ingress ev queue, update last block/pending
        //  pass last block for them to modify
        //   returns last block
        IngressionQueue::insert(chain_id, event_queue);
        LastProcessedBlock::insert(chain_id, last);
        PendingChainBlocks::insert(chain_id, Vec::<ChainBlockTally>::new());
        PendingChainReorgs::insert(chain_id, Vec::<ChainReorgTally>::new());
    } else {
        PendingChainReorgs::insert(chain_id, pending);
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
