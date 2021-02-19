use frame_support::storage::{StorageMap, StorageValue};
use frame_system::offchain::SubmitTransaction;
use sp_runtime::offchain::{
    storage::StorageValueRef,
    storage_lock::{StorageLock, Time},
};

use crate::{
    chains::{Chain, Ethereum},
    log,
    oracle::{open_price_feed_request, parse_message},
    params::OCW_OPEN_ORACLE_POLL_INTERVAL_BLOCKS,
    reason::{OracleError, Reason},
    symbol::Ticker,
    types::{AssetPrice, Timestamp},
    Call, Config, PriceReporters, PriceTimes, Prices,
};
use our_std::str::FromStr;

// OCW storage constants
const OCW_LATEST_PRICE_FEED_TIMESTAMP: &[u8; 33] = b"cash::latest_price_feed_timestamp";
const OCW_LATEST_PRICE_FEED_POLL_BLOCK_NUMBER: &[u8; 41] =
    b"cash::latest_price_feed_poll_block_number";
const OCW_STORAGE_LOCK_OPEN_PRICE_FEED: &[u8; 34] = b"cash::storage_lock_open_price_feed";

pub fn post_price<T: Config>(payload: Vec<u8>, signature: Vec<u8>) -> Result<(), Reason> {
    // check signature
    let parsed_sig: <Ethereum as Chain>::Signature =
        compound_crypto::eth_signature_from_bytes(&signature)?;

    // note that this is actually a double-hash situation but that is expected behavior
    // the hashed message is hashed again in the eth convention inside eth_recover
    let hashed = compound_crypto::keccak(&payload);
    let recovered = compound_crypto::eth_recover(&hashed, &parsed_sig, true)?;
    if !PriceReporters::get().contains(recovered) {
        return Err(OracleError::NotAReporter.into());
    }

    // parse message and check it
    let parsed = parse_message(&payload)?;
    let ticker = Ticker::from_str(&parsed.key)?;

    // XXX
    log!(
        "Parsed price from open price feed: {:?} is worth {:?}",
        parsed.key,
        (parsed.value as f64) / 1000000.0
    );

    // todo: more sanity checking on the value // XXX like what?
    if let Some(last_updated) = PriceTimes::get(&ticker) {
        if parsed.timestamp <= last_updated {
            return Err(OracleError::StalePrice.into());
        }
    }

    // * WARNING begin storage - all checks must happen above * //

    Prices::insert(&ticker, parsed.value as AssetPrice);
    PriceTimes::insert(&ticker, parsed.timestamp as Timestamp);
    Ok(())
}

/// Procedure for offchain worker to processes messages coming out of the open price feed
pub fn process_prices<T: Config>(block_number: T::BlockNumber) -> Result<(), Reason> {
    let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK_OPEN_PRICE_FEED);
    if lock.try_lock().is_err() {
        // working in another thread, no big deal
        return Ok(());
    }

    // get the URL to poll, just return if there is no URL set up
    let url = runtime_interfaces::validator_config_interface::get_opf_url().unwrap_or(vec![]);
    if url.len() == 0 {
        return Ok(());
    }

    // check to see if it is time to poll or not
    let latest_price_feed_poll_block_number_storage =
        StorageValueRef::persistent(OCW_LATEST_PRICE_FEED_POLL_BLOCK_NUMBER);
    if let Some(Some(latest_poll_block_number)) =
        latest_price_feed_poll_block_number_storage.get::<T::BlockNumber>()
    {
        let poll_interval_blocks =
            <T as frame_system::Config>::BlockNumber::from(OCW_OPEN_ORACLE_POLL_INTERVAL_BLOCKS);
        if block_number - latest_poll_block_number < poll_interval_blocks {
            return Ok(());
        }
    }
    let url = String::from_utf8(url).map_err(|_| OracleError::InvalidApiEndpoint)?;

    // poll
    let (messages_and_signatures, timestamp) =
        open_price_feed_request(&url)?.to_message_signature_pairs()?;

    // Check to see if Coinbase api prices were updated or not
    let latest_price_feed_timestamp_storage =
        StorageValueRef::persistent(OCW_LATEST_PRICE_FEED_TIMESTAMP);
    if let Some(Some(latest_price_feed_timestamp)) =
        latest_price_feed_timestamp_storage.get::<String>()
    {
        if latest_price_feed_timestamp == timestamp {
            log!(
                "Open oracle prices for timestamp {:?} has been already posted",
                timestamp
            );
            return Ok(());
        }
    }

    for (msg, sig) in messages_and_signatures {
        // adding some debug info in here, this will become very chatty
        let call = <Call<T>>::post_price(msg, sig);
        SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
            .map_err(|_| OracleError::SubmitError)?;
        // note - there is a log message in check_failure if this extrinsic fails but we should
        // still try to update the other prices even if one extrinsic fails, thus the result
        // is ignored and we continue in this loop
    }
    latest_price_feed_poll_block_number_storage.set(&block_number);
    latest_price_feed_timestamp_storage.set(&timestamp);
    Ok(())
}
