use crate::{
    chains::{
        ChainAsset, ChainBlock, ChainBlockEvent, ChainBlockEvents, ChainBlockNumber,
        ChainBlockTally, ChainBlocks, ChainHash, ChainId, ChainReorg, ChainReorgTally,
        ChainSignature,
    },
    core::{
        self, get_current_validator, get_event_queue, get_last_block, get_validator_set,
        recover_validator, validator_sign,
    },
    debug, error,
    events::{fetch_chain_block, fetch_chain_blocks},
    internal::assets::{get_cash_quantity, get_quantity, get_value},
    log,
    params::{INGRESS_LARGE, INGRESS_QUOTA, INGRESS_SLACK, MIN_EVENT_BLOCKS},
    reason::Reason,
    require,
    types::{CashPrincipalAmount, Quantity, USDQuantity, ValidatorIdentity, USD},
    Call, Config, Event as EventT, IngressionQueue, LastProcessedBlock, Module, PendingChainBlocks,
    PendingChainReorgs,
};
use codec::Encode;
use ethereum_client::EthereumEvent;
use frame_support::storage::StorageMap;
use frame_system::offchain::SubmitTransaction;
use our_std::{cmp::max, collections::btree_set::BTreeSet, convert::TryInto};
use sp_runtime::offchain::{
    storage::StorageValueRef,
    storage_lock::{StorageLock, Time},
};

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

/// Incrementally perform the next step of tracking events from all the underlying chains.
pub fn track_chain_events<T: Config>() -> Result<(), Reason> {
    let mut lock = StorageLock::<Time>::new(b"cash::track_chain_events");
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
    let me = get_current_validator::<T>()?;
    let last_block = get_last_block::<T>(chain_id)?;
    let true_block = fetch_chain_block(chain_id, last_block.number())?;
    if last_block.hash() == true_block.hash() {
        let event_queue = get_event_queue::<T>(chain_id)?;
        let slack = queue_slack(&event_queue);
        let blocks = fetch_chain_blocks(chain_id, last_block.number() + 1, slack)?
            .filter_already_signed(&me.substrate_id, PendingChainBlocks::get(chain_id));
        debug!(
            "Worker sees same fork as chain: {:?}, got new blocks: {:?}",
            true_block, blocks
        );
        memorize_chain_blocks::<T>(&blocks)?;
        submit_chain_blocks::<T>(&blocks)
    } else {
        debug!(
            "Worker sees a different fork: {:?} {:?}",
            true_block, last_block
        );
        let reorg = formulate_reorg::<T>(&last_block, &true_block)?;
        if !already_submitted_reorg::<T>(&me.substrate_id, &reorg) {
            submit_chain_reorg::<T>(&reorg)
        } else {
            debug!("Worker already submitted...waiting");
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
        if block_num >= event.block_number() + MIN_EVENT_BLOCKS {
            match risk_adjusted_value::<T>(event, block_num) {
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
    last_block: &ChainBlock,
    true_block: &ChainBlock,
) -> Result<ChainReorg, Reason> {
    const CHUNK: u32 = 10;
    let mut reverse_blocks: Vec<ChainBlock> = vec![];
    let mut drawrof_blocks: Vec<ChainBlock> = vec![];
    let mut reverse_hashes = BTreeSet::<ChainHash>::new();
    let common_ancestor = 'search: loop {
        let reverse_blocks_next = recall_chain_blocks::<T>(last_block.hash(), CHUNK)?;
        let drawrof_blocks_next = recall_chain_blocks::<T>(true_block.hash(), CHUNK)?;
        if reverse_blocks_next.len() == 0 && drawrof_blocks_next.len() == 0 {
            return Err(Reason::CannotFormulateReorg);
        }
        for block in reverse_blocks_next {
            reverse_blocks.push(block.clone());
            reverse_hashes.insert(block.hash());
        }
        for block in drawrof_blocks_next {
            let parent_hash = block.parent_hash();
            if reverse_hashes.contains(&parent_hash) {
                break 'search parent_hash;
            } else {
                drawrof_blocks.push(block);
            }
        }
    };

    match (last_block.hash(), true_block.hash()) {
        (ChainHash::Eth(from_hash), ChainHash::Eth(to_hash)) => Ok(ChainReorg::Eth {
            from_hash,
            to_hash,
            reverse_blocks: reverse_blocks
                .into_iter()
                .take_while(|b| b.parent_hash() != common_ancestor)
                .filter_map(|b| match b {
                    ChainBlock::Eth(eth_block) => Some(eth_block),
                })
                .collect(),
            forward_blocks: drawrof_blocks
                .into_iter()
                .take_while(|b| b.parent_hash() != common_ancestor)
                .filter_map(|b| match b {
                    ChainBlock::Eth(eth_block) => Some(eth_block),
                })
                .collect_rev(),
        }),

        _ => panic!("XXX not implemented"),
    }
}

/// Check whether the given validator already submitted the given reorg.
pub fn already_submitted_reorg<T: Config>(id: &ValidatorIdentity, reorg: &ChainReorg) -> bool {
    // XXX dont resubmit for same thing
    false
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
    for block in blocks.blocks() {
        debug!("Event queue: {:?}", event_queue);
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
                parent_hash: [
                    97, 49, 76, 28, 104, 55, 225, 94, 96, 197, 182, 115, 47, 9, 33, 24, 221, 37,
                    227, 236, 104, 31, 94, 8, 155, 58, 154, 210, 55, 78, 90, 138,
                ],
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

    // XXX test reorg, etc
}
