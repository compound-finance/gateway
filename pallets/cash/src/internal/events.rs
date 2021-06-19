use crate::{
    chains::{
        ChainAsset, ChainBlock, ChainBlockEvent, ChainBlockEvents, ChainBlockNumber,
        ChainBlockTally, ChainBlocks, ChainHash, ChainId, ChainReorg, ChainReorgTally,
        ChainSignature,
    },
    core::{
        self, get_current_validator, get_event_queue, get_last_block, get_starport,
        get_validator_set, recover_validator, validator_sign,
    },
    debug, error,
    events::{fetch_chain_block, fetch_chain_blocks},
    internal::assets::{get_cash_quantity, get_quantity, get_value},
    log,
    params::{
        INGRESS_LARGE, INGRESS_QUOTA, INGRESS_SLACK, MAX_EVENT_BLOCKS, MIN_EVENT_BLOCKS,
        REORG_CHUNK_SIZE,
    },
    reason::Reason,
    require,
    types::{CashPrincipalAmount, Quantity, USDQuantity, USD},
    Call, Config, Event as EventT, IngressionQueue, LastProcessedBlock, Module, PendingChainBlocks,
    PendingChainReorgs,
};
use codec::Encode;
use ethereum_client::EthereumEvent;
use frame_support::storage::StorageMap;
use frame_system::offchain::SubmitTransaction;
use our_std::{
    cmp::max,
    collections::btree_set::BTreeSet,
    convert::TryInto,
    sync::atomic::{AtomicU8, Ordering},
};
use sp_runtime::offchain::storage::StorageValueRef;

trait CollectRev: Iterator {
    fn collect_rev(self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        let mut v: Vec<Self::Item> = self.into_iter().collect();
        v.reverse();
        v
    }
}

impl<I: Iterator> CollectRev for I {}

/// Determine the number of blocks which can still fit on an ingression queue.
pub fn queue_slack(event_queue: &ChainBlockEvents) -> u32 {
    let queue_len: u32 = event_queue.len().try_into().unwrap_or(u32::MAX);
    max(INGRESS_SLACK.saturating_sub(queue_len), 1)
}

/// Determine the risk-adjusted value of a particular event, given the current block number.
pub fn risk_adjusted_value<T: Config>(
    block_event: &ChainBlockEvent,
    block_number: ChainBlockNumber,
) -> Result<USDQuantity, Reason> {
    let elapsed_blocks = block_number
        .checked_sub(block_event.block_number())
        .ok_or(Reason::Unreachable)?;
    match block_event {
        ChainBlockEvent::Reserved => panic!("reserved"),
        ChainBlockEvent::Eth(_block_num, eth_event) => match eth_event {
            EthereumEvent::Lock { asset, amount, .. } => {
                let quantity = get_quantity::<T>(ChainAsset::Eth(*asset), *amount)?;
                let usd_quantity = get_value::<T>(quantity)?;
                Ok(usd_quantity.decay(elapsed_blocks)?)
            }

            EthereumEvent::LockCash { principal, .. } => {
                let quantity = get_cash_quantity::<T>(CashPrincipalAmount(*principal))?;
                let usd_quantity = get_value::<T>(quantity)?;
                Ok(usd_quantity.decay(elapsed_blocks)?)
            }

            EthereumEvent::ExecuteProposal { .. } => {
                let usd_quantity = get_value::<T>(INGRESS_LARGE)?;
                Ok(usd_quantity.decay(elapsed_blocks)?)
            }

            _ => Ok(Quantity::new(0, USD)),
        },
    }
}

static TRACK_CHAIN_EVENTS_LOCK: AtomicU8 = AtomicU8::new(0);

/// Incrementally perform the next step of tracking events from all the underlying chains.
pub fn track_chain_events<T: Config>() -> Result<(), Reason> {
    if TRACK_CHAIN_EVENTS_LOCK.fetch_add(1, Ordering::SeqCst) == 0 {
        // Note: chains could be parallelized
        let result = track_chain_events_on::<T>(ChainId::Eth);
        TRACK_CHAIN_EVENTS_LOCK.store(0, Ordering::SeqCst);
        result
    } else {
        Err(Reason::WorkerBusy)
    }
}

/// Perform the next step of tracking events from an underlying chain.
pub fn track_chain_events_on<T: Config>(chain_id: ChainId) -> Result<(), Reason> {
    let starport = get_starport::<T>(chain_id)?;
    let me = get_current_validator::<T>()?;
    let last_block = get_last_block::<T>(chain_id)?;
    let true_block = fetch_chain_block(chain_id, last_block.number(), starport)?;
    if last_block.hash() == true_block.hash() {
        let pending_blocks = PendingChainBlocks::get(chain_id);
        let event_queue = get_event_queue::<T>(chain_id)?;
        let slack = queue_slack(&event_queue) as u64;
        let blocks = fetch_chain_blocks(
            chain_id,
            last_block.number() + 1,
            last_block.number() + 1 + slack,
            starport,
        )?
        .filter_already_supported(&me.substrate_id, pending_blocks);
        memorize_chain_blocks::<T>(&blocks)?;
        submit_chain_blocks::<T>(&blocks)
    } else {
        debug!(
            "Worker sees a different fork: true={:?} last={:?}",
            true_block, last_block
        );
        let pending_reorgs = PendingChainReorgs::get(chain_id);
        let reorg = formulate_reorg::<T>(chain_id, &last_block, &true_block, REORG_CHUNK_SIZE)?;
        if !reorg.is_already_signed(&me.substrate_id, pending_reorgs) {
            submit_chain_reorg::<T>(&reorg)
        } else {
            debug!("Worker already submitted... waiting");
            // just wait for the reorg to succeed or fail,
            //  or we change our minds in another pass (noop)
            Ok(())
        }
    }
}

/// Ingress a single round (quota per underlying chain block ingested).
pub fn ingress_queue<T: Config>(
    last_block: &ChainBlock,
    event_queue: &mut ChainBlockEvents,
) -> Result<(), Reason> {
    let mut available = INGRESS_QUOTA;
    let block_num = last_block.number();

    event_queue.retain(|event| {
        let delta_blocks = block_num.saturating_sub(event.block_number());

        if delta_blocks >= MIN_EVENT_BLOCKS {
            // If we're beyond max risk block, then simply accept event
            let risk_result = if delta_blocks > MAX_EVENT_BLOCKS {
                Ok(Quantity::new(0, USD))
            } else {
                risk_adjusted_value::<T>(event, block_num)
            };

            match risk_result {
                Ok(value) => {
                    debug!(
                        "Computed risk adjusted value ({:?} / {:?} @ {}) of {:?}",
                        value, available, block_num, event
                    );
                    if value <= available {
                        available = available.sub(value).unwrap();
                        match core::apply_chain_event_internal::<T>(event) {
                            Ok(()) => {
                                <Module<T>>::deposit_event(EventT::ProcessedChainBlockEvent(
                                    event.clone(),
                                ));
                            }

                            Err(reason) => {
                                <Module<T>>::deposit_event(
                                    EventT::FailedProcessingChainBlockEvent(event.clone(), reason),
                                );
                            }
                        }
                        return false; // remove from queue
                    } else {
                        return true; // retain on queue
                    }
                }

                Err(err) => {
                    error!(
                        "Could not compute risk adjusted value ({}) of {:?}",
                        err, event
                    );
                    // note that we keep the event if we cannot compute the risk adjusted value,
                    //  there's not an obviously more reasonable thing to do right now
                    // there's no reason this should fail normally but it can
                    //  e.g. if somehow someone locks an unsupported asset in the starport
                    // we need to take separate measures to forcefully limit the queue size
                    //  e.g. reject new blocks once the event queue reaches a certain size
                    return true; // retain on queue
                }
            }
        } else {
            debug!(
                "Event not old enough to process (@ {:?}) {:?}",
                block_num, event
            );
            return true; // retain on queue
        }
    });
    Ok(())
}

/// Submit the underlying chain blocks the worker calculates are needed by the chain next.
pub fn submit_chain_blocks<T: Config>(blocks: &ChainBlocks) -> Result<(), Reason> {
    if blocks.len() > 0 {
        log!("Submitting chain blocks extrinsic: {:?}", blocks);
        let signature = validator_sign::<T>(&blocks.encode()[..])?;
        let call = Call::receive_chain_blocks(blocks.clone(), signature);
        if let Err(e) = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()) {
            log!("Error while submitting chain blocks: {:?}", e);
            return Err(Reason::FailedToSubmitExtrinsic);
        }
    }
    Ok(())
}

/// Remember whatever blocks we submit, so we can formulate reorgs if needed.
pub fn memorize_chain_blocks<T: Config>(blocks: &ChainBlocks) -> Result<(), Reason> {
    // Note: grows unboundedly, but pruning history can happen independently / later
    for block in blocks.blocks() {
        let key = format!("cash::memorize_chain_blocks::{}", block.hash());
        let krf = StorageValueRef::persistent(key.as_bytes());
        krf.set(&block);
    }
    Ok(())
}

/// Walk backwards through the locally stored blocks, in order to formulate a reorg path.
pub fn recall_chain_blocks<T: Config>(
    start_hash: ChainHash,
    num_blocks: u32,
) -> Result<Vec<ChainBlock>, Reason> {
    let mut blocks: Vec<ChainBlock> = vec![];
    let mut key = format!("cash::memorize_chain_blocks::{}", start_hash);
    let mut krf = StorageValueRef::persistent(key.as_bytes());
    for _ in 0..num_blocks {
        match krf.get::<ChainBlock>() {
            Some(Some(block)) => {
                key = format!("cash::memorize_chain_blocks::{}", block.parent_hash());
                krf = StorageValueRef::persistent(key.as_bytes());
                blocks.push(block);
            }

            _ => break,
        }
    }
    Ok(blocks)
}

/// Try to form a path from the last block to the new true block.
pub fn formulate_reorg<T: Config>(
    chain_id: ChainId,
    last_block: &ChainBlock,
    true_block: &ChainBlock,
    chunk_size: u32,
) -> Result<ChainReorg, Reason> {
    let starport = get_starport::<T>(chain_id)?;
    let mut reverse_blocks: Vec<ChainBlock> = vec![]; // reverse blocks in correct order
    let mut drawrof_blocks: Vec<ChainBlock> = vec![]; // forward blocks in reverse order
    let mut reverse_hashes = BTreeSet::<ChainHash>::new();
    let mut reverse_blocks_hash_next = Some(last_block.hash());
    let mut drawrof_blocks_number_next = true_block.number().checked_add(1); // exclusive
    let common_ancestor = 'search: loop {
        // fetch the next batch of blocks, going backwards further each time
        //  these are the reverse_blocks used by the reorg in the expected order
        let reverse_blocks_next = match reverse_blocks_hash_next {
            Some(hash) => recall_chain_blocks::<T>(hash, chunk_size)?,
            None => Vec::new(),
        };

        // note that `drawrof_blocks_next` *could* be built directly in reverse order
        //  these are the forward_blocks used by the reorg, but backwards
        let drawrof_blocks_next = match drawrof_blocks_number_next {
            Some(number) => fetch_chain_blocks(
                true_block.chain_id(),
                number.saturating_sub(chunk_size as u64),
                number,
                starport,
            )?
            .blocks()
            .collect_rev(),
            None => Vec::new(),
        };

        // it's an error if we can't make progress pulling more forward or reverse blocks
        if reverse_blocks_next.len() == 0 && drawrof_blocks_next.len() == 0 {
            return Err(Reason::CannotFormulateReorg);
        }

        // be sure to pick up where we left off on the next go round
        reverse_blocks_hash_next = reverse_blocks_next.last().map(|b| b.parent_hash());
        drawrof_blocks_number_next = drawrof_blocks_next.last().map(|b| b.number());

        // just add all the reverse blocks to the full list, and index their hashes
        for block in reverse_blocks_next {
            reverse_blocks.push(block.clone());
            reverse_hashes.insert(block.hash());
        }

        // note that the following is correct if the original blocks are at the same height
        //  however that fact also makes this entire algorithm obsolete / inefficient
        //   we can simply compare parents at the same height one at a time
        //  but for this to be generally correct, we would need to recheck all `drawrof_blocks`
        for block in drawrof_blocks_next {
            let parent_hash = block.parent_hash();
            drawrof_blocks.push(block);
            if reverse_hashes.contains(&parent_hash) {
                break 'search parent_hash;
            }
        }
    };

    match (last_block.hash(), true_block.hash()) {
        (ChainHash::Eth(from_hash), ChainHash::Eth(to_hash)) => Ok(ChainReorg::Eth {
            from_hash,
            to_hash,
            reverse_blocks: reverse_blocks
                .into_iter()
                .take_while(|b| b.hash() != common_ancestor)
                .filter_map(|b| match b {
                    ChainBlock::Eth(eth_block) => Some(eth_block),
                })
                .collect(),
            forward_blocks: drawrof_blocks
                .into_iter()
                .filter_map(|b| match b {
                    ChainBlock::Eth(eth_block) => Some(eth_block),
                })
                .collect_rev(),
        }),

        _ => return Err(Reason::Unreachable),
    }
}

/// Submit a reorg message from a worker to the chain.
pub fn submit_chain_reorg<T: Config>(reorg: &ChainReorg) -> Result<(), Reason> {
    log!("Submitting chain reorg extrinsic: {:?}", reorg);
    let signature = validator_sign::<T>(&reorg.encode()[..])?;
    let call = Call::receive_chain_reorg(reorg.clone(), signature);
    if let Err(e) = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()) {
        log!("Error while submitting chain blocks: {:?}", e);
        return Err(Reason::FailedToSubmitExtrinsic);
    }
    Ok(())
}

/// Receive a blocks message from a worker, tallying it and applying as necessary.
pub fn receive_chain_blocks<T: Config>(
    blocks: ChainBlocks,
    signature: ChainSignature,
) -> Result<(), Reason> {
    let validator_set = get_validator_set::<T>()?;
    let validator = recover_validator::<T>(&blocks.encode(), signature)?;
    let chain_id = blocks.chain_id();
    let mut event_queue = get_event_queue::<T>(chain_id)?;
    let mut last_block = get_last_block::<T>(chain_id)?;
    let mut pending_blocks = PendingChainBlocks::get(chain_id);

    debug!("Pending blocks: {:?}", pending_blocks);
    debug!("Event queue: {:?}", event_queue);

    for block in blocks.blocks() {
        if block.number() >= last_block.number() + 1 {
            let offset = (block.number() - last_block.number() - 1) as usize;
            if let Some(prior) = pending_blocks.get_mut(offset) {
                if block != prior.block {
                    debug!(
                        "Received conflicting block, dissenting: {:?} ({:?})",
                        block, prior
                    );
                    prior.add_dissent(&validator);
                } else {
                    debug!("Received support for existing block: {:?}", block);
                    prior.add_support(&validator);
                }
            } else if offset == 0 {
                if block.parent_hash() != last_block.hash() {
                    debug!(
                        "Received block which would require fork: {:?} ({:?})",
                        block, last_block
                    );
                    // worker should reorg if it wants to build something else
                    //  but could just be a laggard message
                    // this *would* be a dissenting vote if prior existed
                    //  but that's ok bc worker will try to reorg instead
                    continue;
                } else {
                    debug!("Received valid first next pending block: {:?}", block);
                    // write to pending_blocks[offset]
                    //  we already checked offset doesn't exist, this is the first element
                    pending_blocks.push(ChainBlockTally::new(block, &validator));
                }
            } else if let Some(parent) = pending_blocks.get(offset - 1) {
                if block.parent_hash() != parent.block.hash() {
                    debug!(
                        "Received invalid derivative block: {:?} ({:?})",
                        block, parent
                    );
                    // worker is submitting block for parent that conflicts
                    //  if its also submitting the parent, which it should be,
                    //   that one will vote against and propagate forwards
                    // this *would* be a dissenting vote if prior existed
                    //  but that's ok bc worker should submit parent first
                    continue;
                } else {
                    debug!("Received valid pending block: {:?}", block);
                    // write to pending_blocks[offset]
                    //  we already checked offset doesn't exist, but offset - 1 does
                    pending_blocks.push(ChainBlockTally::new(block, &validator));
                }
            } else {
                debug!("Received disconnected block: {:?} ({:?})", block, offset);
                // we don't have the block, nor a parent for it
                //  the worker shouldn't submit stuff like this
                // blocks should be in order in which case this wouldn't happen
                //  but just ignore, if workers aren't connecting blocks we won't make progress
                continue;
            }
        } else {
            debug!(
                "Received irrelevant past block: {:?} ({:?})",
                block, last_block
            );
            continue;
        }
    }

    for tally in pending_blocks.clone().iter() {
        if tally.has_enough_support(&validator_set) {
            // remove tally from block queue
            //  add events to event queue, advance the block, and process a round of events
            pending_blocks.remove(0); // note: tally is first on queue
            event_queue.push(&tally.block);
            last_block = tally.block.clone();
            ingress_queue::<T>(&last_block, &mut event_queue)?;
            continue;
        } else if tally.has_enough_dissent(&validator_set) {
            // remove tally and everything after from queue
            pending_blocks = vec![];
            break;
        } else {
            break;
        }
    }

    LastProcessedBlock::insert(chain_id, last_block);
    PendingChainBlocks::insert(chain_id, pending_blocks);
    IngressionQueue::insert(chain_id, event_queue);

    Ok(())
}

/// Receive a reorg message from a worker, tallying it and applying as necessary.
pub fn receive_chain_reorg<T: Config>(
    reorg: ChainReorg,
    signature: ChainSignature,
) -> Result<(), Reason> {
    let validator_set = get_validator_set::<T>()?;
    let validator = recover_validator::<T>(&reorg.encode(), signature)?;
    let chain_id = reorg.chain_id();
    let mut event_queue = get_event_queue::<T>(chain_id)?;
    let mut last_block = get_last_block::<T>(chain_id)?;
    let mut pending_reorgs = PendingChainReorgs::get(chain_id);

    // Note: can reject / stop propagating once this check fails
    require!(reorg.from_hash() == last_block.hash(), Reason::HashMismatch);

    let tally = if let Some(prior) = pending_reorgs.iter_mut().find(|r| r.reorg == reorg) {
        prior.add_support(&validator);
        prior
    } else {
        pending_reorgs.push(ChainReorgTally::new(chain_id, reorg, &validator));
        pending_reorgs.last_mut().unwrap()
    };

    // Note: whenever there's a race to be the last signer, this will be suboptimal
    //  we don't currently keep a tombstone marking that the reorg was recently processed
    if tally.has_enough_support(&validator_set) {
        // if we have enough support, perform actual reorg
        // for each block going backwards
        //  remove events from queue, or unapply them if already applied
        for block in tally.reorg.reverse_blocks() {
            for event in block.events() {
                // Note: this could be made significantly more efficient
                //  at the cost of significant complexity
                if let Some(pos) = event_queue.position(&event) {
                    event_queue.remove(pos);
                } else {
                    core::unapply_chain_event_internal::<T>(&event)?
                }
            }
        }

        // for each block going forwards
        //  add events to event queue, advance the block, and process a round of events
        for block in tally.reorg.forward_blocks() {
            event_queue.push(&block);
            last_block = block.clone();
            ingress_queue::<T>(&last_block, &mut event_queue)?;
        }

        // write the new state back to storage
        LastProcessedBlock::insert(chain_id, last_block);
        PendingChainBlocks::insert(chain_id, Vec::<ChainBlockTally>::new());
        PendingChainReorgs::insert(chain_id, Vec::<ChainReorgTally>::new());
        IngressionQueue::insert(chain_id, event_queue);
    } else {
        // otherwise just update the stored reorg tallies
        PendingChainReorgs::insert(chain_id, pending_reorgs);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
    use ethereum_client::EthereumBlock;

    fn gen_blocks(start_block: u64, end_block: u64, pad: u8) -> Vec<EthereumBlock> {
        let mut hash = [0u8; 32];
        let mut v: Vec<ethereum_client::EthereumBlock> = vec![];
        for i in start_block..end_block {
            let parent_hash = hash;
            let mut hashvec = i.to_le_bytes().to_vec();
            hashvec.extend_from_slice(&[pad; 24]);
            hash = hashvec.try_into().unwrap();
            v.push(EthereumBlock {
                hash,
                parent_hash,
                number: i,
                events: vec![],
            });
        }
        return v;
    }

    #[test]
    fn test_formulate_reorg() {
        const CHUNK: u64 = REORG_CHUNK_SIZE as u64;
        const STARPORT_ADDR: [u8; 20] = [0x77; 20];

        // mock the exact chunk of blocks in fetch_eth_blocks in the order they are called
        let common_ancestor_idx = CHUNK - 1;
        let last_block_idx = CHUNK;
        let mut prev_blocks: Vec<EthereumBlock> =
            gen_blocks(last_block_idx - CHUNK + 1, common_ancestor_idx, 0);
        let reorg_hash = [5; 32];
        let ancestor_hash = [3; 32];
        let true_block_hash = [8; 32];

        let commmon_ancestor_block = ethereum_client::EthereumBlock {
            hash: ancestor_hash.clone(),
            parent_hash: [100; 32],
            number: common_ancestor_idx,
            events: vec![],
        };

        let last_block = ethereum_client::EthereumBlock {
            hash: reorg_hash.clone(),
            parent_hash: commmon_ancestor_block.hash,
            number: last_block_idx,
            events: vec![],
        };

        prev_blocks.push(commmon_ancestor_block.clone());

        let mut true_chain = prev_blocks.clone();

        let true_block = ethereum_client::EthereumBlock {
            hash: true_block_hash.clone(),
            parent_hash: commmon_ancestor_block.hash,
            number: last_block_idx,
            events: vec![],
        };

        true_chain.push(true_block.clone());
        prev_blocks.push(last_block.clone());

        let calls = gen_mock_calls(&true_chain, STARPORT_ADDR);
        let (mut t, _, _) = new_test_ext_with_http_calls(calls);

        t.execute_with(|| {
            initialize_storage();
            memorize_chain_blocks::<Test>(&ChainBlocks::Eth(prev_blocks)).unwrap();
            LastProcessedBlock::insert(ChainId::Eth, ChainBlock::Eth(last_block.clone()));
            let reorg = formulate_reorg::<Test>(
                ChainId::Eth,
                &ChainBlock::Eth(last_block.clone()),
                &ChainBlock::Eth(true_block.clone()),
                CHUNK as u32,
            )
            .unwrap();

            match reorg {
                ChainReorg::Eth {
                    from_hash,
                    to_hash,
                    reverse_blocks,
                    forward_blocks,
                } => {
                    assert_eq!(from_hash, reorg_hash);
                    assert_eq!(to_hash, true_block_hash);
                    assert_eq!(reverse_blocks, vec![last_block]);
                    assert_eq!(
                        forward_blocks.iter().map(|x| x.hash).collect::<Vec<_>>(),
                        vec![true_block.hash]
                    );
                }
            }
        });
    }

    #[test]
    fn test_formulate_reorg_multiple_chunks() {
        const CHUNK: u64 = 5 as u64;
        const STARPORT_ADDR: [u8; 20] = [0x77; 20];

        // mock the exact chunk of blocks in fetch_eth_blocks in the order they are called
        let old_chain: Vec<EthereumBlock> = gen_blocks(0, 10, 0);
        let new_chain: Vec<EthereumBlock> = gen_blocks(1, 10, 1);
        let common_ancestor_block = old_chain[0].clone();
        let last_block = old_chain.last().unwrap().clone();
        let true_block = new_chain.last().unwrap().clone();

        let mut fetched_blocks = vec![];
        fetched_blocks.extend_from_slice(&new_chain[4..9]);
        fetched_blocks.push(common_ancestor_block);
        fetched_blocks.extend_from_slice(&new_chain[0..4]);
        let calls = gen_mock_calls(&fetched_blocks, STARPORT_ADDR);
        let (mut t, _, _) = new_test_ext_with_http_calls(calls);

        t.execute_with(|| {
            initialize_storage();
            memorize_chain_blocks::<Test>(&ChainBlocks::Eth(old_chain.clone())).unwrap();
            LastProcessedBlock::insert(ChainId::Eth, ChainBlock::Eth(last_block.clone()));
            let reorg = formulate_reorg::<Test>(
                ChainId::Eth,
                &ChainBlock::Eth(last_block.clone()),
                &ChainBlock::Eth(true_block.clone()),
                CHUNK as u32,
            )
            .unwrap();

            match reorg {
                ChainReorg::Eth {
                    from_hash,
                    to_hash,
                    reverse_blocks,
                    forward_blocks,
                } => {
                    assert_eq!(from_hash, last_block.hash);
                    assert_eq!(to_hash, true_block.hash);
                    assert_eq!(
                        reverse_blocks,
                        old_chain[1..10].iter().rev().cloned().collect::<Vec<_>>()
                    );
                    assert_eq!(
                        forward_blocks.iter().map(|x| x.hash).collect::<Vec<_>>(),
                        new_chain.iter().map(|x| x.hash).collect::<Vec<_>>()
                    );
                }
            }
        });
    }

    #[test]
    fn test_receive_chain_reorg() -> Result<(), Reason> {
        new_test_ext().execute_with(|| {
            initialize_storage();
            pallet_oracle::Prices::insert(
                ETH.ticker,
                Price::from_nominal(ETH.ticker, "2000.00").value,
            );

            let reorg_block_hash = [3; 32];
            let real_block_hash = [5; 32];

            let reorg_event = EthereumEvent::Lock {
                asset: [238; 20],
                sender: [3; 20],
                chain: String::from("ETH"),
                recipient: [4; 32],
                amount: qty!("10", ETH).value,
            };

            let real_event = EthereumEvent::Lock {
                sender: [3; 20],
                chain: String::from("ETH"),
                recipient: [5; 32],
                amount: qty!("9", ETH).value,
                asset: [238; 20],
            };

            let reorg_block = ethereum_client::EthereumBlock {
                hash: reorg_block_hash,
                parent_hash: premined_block().hash,
                number: 2,
                events: vec![reorg_event.clone()],
            };

            let real_block = ethereum_client::EthereumBlock {
                hash: real_block_hash,
                parent_hash: premined_block().hash,
                number: 2,
                events: vec![real_event.clone()],
            };

            let latest_hash = [10; 32];

            // mine dummy blocks to get past limit
            let blocks_3 = ChainBlocks::Eth(vec![
                ethereum_client::EthereumBlock {
                    hash: [3; 32],
                    parent_hash: reorg_block_hash,
                    number: 3,
                    events: vec![],
                },
                ethereum_client::EthereumBlock {
                    hash: [4; 32],
                    parent_hash: [3; 32],
                    number: 4,
                    events: vec![],
                },
                ethereum_client::EthereumBlock {
                    hash: latest_hash,
                    parent_hash: [4; 32],
                    number: 5,
                    events: vec![],
                },
            ]);

            let reorg = ChainReorg::Eth {
                from_hash: latest_hash,
                to_hash: real_block_hash,
                reverse_blocks: vec![reorg_block.clone()],
                forward_blocks: vec![real_block.clone()],
            };

            // apply the to-be reorg'd block and a dummy block so that it is ingressed, show that the event was applied
            assert_ok!(all_receive_chain_blocks(&ChainBlocks::Eth(vec![
                reorg_block
            ])));
            let event_queue = get_event_queue::<Test>(ChainId::Eth)?;
            assert_eq!(event_queue, ChainBlockEvents::Eth(vec![(2, reorg_event)]));

            assert_ok!(all_receive_chain_blocks(&blocks_3));
            assert_eq!(
                AssetBalances::get(&Eth, ChainAccount::Eth([4; 20])),
                bal!("10", ETH).value
            );
            assert_eq!(PendingChainReorgs::get(ChainId::Eth), vec![]);

            // val a sends reorg, tally started
            assert_ok!(a_receive_chain_reorg(&reorg), ());
            assert_eq!(
                PendingChainReorgs::get(ChainId::Eth),
                vec![ChainReorgTally::new(ChainId::Eth, reorg.clone(), &val_a())]
            );

            // val b sends reorg and show reorg is executed and the new event is applied and the old one is reverted
            assert_ok!(b_receive_chain_reorg(&reorg), ());
            assert_eq!(
                LastProcessedBlock::get(ChainId::Eth),
                Some(ChainBlock::Eth(real_block))
            );
            assert_eq!(
                PendingChainBlocks::get(ChainId::Eth),
                Vec::<ChainBlockTally>::new()
            );
            assert_eq!(PendingChainReorgs::get(ChainId::Eth), vec![]);
            let event_queue = get_event_queue::<Test>(ChainId::Eth)?;
            assert_eq!(
                event_queue,
                ChainBlockEvents::Eth(vec![(2, real_event.clone())])
            );

            // mine a block so that block is ingressed
            // mine dummy blocks to get past limit
            let blocks_4 = ChainBlocks::Eth(vec![
                ethereum_client::EthereumBlock {
                    hash: [3; 32],
                    parent_hash: real_block_hash,
                    number: 3,
                    events: vec![],
                },
                ethereum_client::EthereumBlock {
                    hash: [4; 32],
                    parent_hash: [3; 32],
                    number: 4,
                    events: vec![],
                },
                ethereum_client::EthereumBlock {
                    hash: [5; 32],
                    parent_hash: [4; 32],
                    number: 5,
                    events: vec![],
                },
            ]);
            assert_ok!(all_receive_chain_blocks(&blocks_4));

            assert_eq!(
                AssetBalances::get(&Eth, ChainAccount::Eth([5; 20])),
                bal!("9", ETH).value
            );

            assert_eq!(
                AssetBalances::get(&Eth, ChainAccount::Eth([4; 20])),
                bal!("0", ETH).value
            );

            Ok(())
        })
    }

    #[test]
    fn test_collect_rev() {
        let x = vec![1, 2, 3];
        let y = x.iter().map(|v| v + 1).collect_rev();
        assert_eq!(y, vec![4, 3, 2]);
    }

    #[test]
    fn test_receive_chain_blocks_fails_for_signed_origin() {
        new_test_ext().execute_with(|| {
            let blocks = ChainBlocks::Eth(vec![]);
            let signature = ChainSignature::Eth([0u8; 65]);
            assert_err!(
                CashModule::receive_chain_blocks(
                    Origin::signed(Default::default()),
                    blocks,
                    signature
                ),
                DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn test_receive_chain_blocks_fails_for_invalid_signature() {
        new_test_ext().execute_with(|| {
            let blocks = ChainBlocks::Eth(vec![]);
            let signature = ChainSignature::Eth([0u8; 65]);
            assert_err!(
                CashModule::receive_chain_blocks(Origin::none(), blocks, signature),
                Reason::CryptoError(gateway_crypto::CryptoError::RecoverError),
            );
        });
    }

    #[test]
    fn test_receive_chain_blocks_fails_if_not_validator() {
        new_test_ext().execute_with(|| {
            let blocks = ChainBlocks::Eth(vec![]);
            let signature = ChainId::Eth.sign(&blocks.encode()).unwrap();
            assert_err!(
                CashModule::receive_chain_blocks(Origin::none(), blocks, signature),
                Reason::UnknownValidator
            );
        });
    }

    #[test]
    fn test_receive_chain_blocks_happy_path() -> Result<(), Reason> {
        sp_tracing::try_init_simple(); // Tip: add/remove from tests for logging

        new_test_ext().execute_with(|| {
            initialize_storage();

            pallet_oracle::Prices::insert(
                ETH.ticker,
                Price::from_nominal(ETH.ticker, "2000.00").value,
            );

            let event = ethereum_client::EthereumEvent::Lock {
                asset: [238; 20],
                sender: [3; 20],
                chain: String::from("ETH"),
                recipient: [2; 32],
                amount: qty!("75", ETH).value,
            };
            let blocks_2 = ChainBlocks::Eth(vec![ethereum_client::EthereumBlock {
                hash: [2; 32],
                parent_hash: premined_block().hash,
                number: 2,
                events: vec![event.clone()],
            }]);
            let blocks_3 = ChainBlocks::Eth(vec![
                ethereum_client::EthereumBlock {
                    hash: [3; 32],
                    parent_hash: [2; 32],
                    number: 3,
                    events: vec![],
                },
                ethereum_client::EthereumBlock {
                    hash: [4; 32],
                    parent_hash: [3; 32],
                    number: 4,
                    events: vec![],
                },
                ethereum_client::EthereumBlock {
                    hash: [5; 32],
                    parent_hash: [4; 32],
                    number: 5,
                    events: vec![],
                },
            ]);
            let blocks_4 = ChainBlocks::Eth(vec![ethereum_client::EthereumBlock {
                hash: [6; 32],
                parent_hash: [5; 32],
                number: 6,
                events: vec![],
            }]);

            assert_eq!(
                AssetBalances::get(&Eth, ChainAccount::Eth([2; 20])),
                bal!("0", ETH).value
            );

            // Sign and dispatch from first validator
            assert_ok!(a_receive_chain_blocks(&blocks_2));

            // Block should be pending, nothing in event queue yet
            let pending_blocks = PendingChainBlocks::get(ChainId::Eth);
            assert_eq!(pending_blocks.len(), 1);
            let event_queue = get_event_queue::<Test>(ChainId::Eth)?;
            assert_eq!(event_queue, ChainBlockEvents::Eth(vec![]));

            // Sign and dispatch from second validator
            assert_ok!(b_receive_chain_blocks(&blocks_2));

            // First round is too new - not yet processed
            let pending_blocks = PendingChainBlocks::get(ChainId::Eth);
            assert_eq!(pending_blocks.len(), 0);
            let event_queue = get_event_queue::<Test>(ChainId::Eth)?;
            assert_eq!(event_queue, ChainBlockEvents::Eth(vec![(2, event)]));

            // Receive enough blocks to ingress another round
            assert_ok!(all_receive_chain_blocks(&blocks_3));

            // Second round should still be over quota - not yet processed
            let pending_blocks = PendingChainBlocks::get(ChainId::Eth);
            assert_eq!(pending_blocks.len(), 0);
            let event_queue = get_event_queue::<Test>(ChainId::Eth)?;
            assert_eq!(event_queue.len(), 1);

            // Receive enough blocks to ingress another round
            assert_ok!(all_receive_chain_blocks(&blocks_4));

            // Third round should process
            let pending_blocks = PendingChainBlocks::get(ChainId::Eth);
            assert_eq!(pending_blocks.len(), 0);
            let event_queue = get_event_queue::<Test>(ChainId::Eth)?;
            assert_eq!(event_queue.len(), 0);

            // Check the final balance
            assert_eq!(
                AssetBalances::get(&Eth, ChainAccount::Eth([2; 20])),
                bal!("75", ETH).value
            );

            Ok(())
        })
    }
}
