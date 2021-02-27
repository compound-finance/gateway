use frame_support::storage::{StorageMap, StorageValue};
use frame_system::offchain::SubmitTransaction;
use serde::Deserialize;
use sp_runtime::offchain::{
    http,
    storage::StorageValueRef,
    storage_lock::{StorageLock, Time},
    Duration,
};

use crate::{
    chains::{Chain, Ethereum},
    log,
    params::ORACLE_POLL_INTERVAL_BLOCKS,
    reason::{OracleError, Reason},
    symbol::Ticker,
    types::{AssetPrice, Timestamp},
    Call, Config, PriceReporters, PriceTimes, Prices,
};
use our_std::{collections::btree_map::BTreeMap, str::FromStr, vec::Vec, RuntimeDebug};

/// A single decoded message from the price oracle
#[derive(PartialEq, Eq, RuntimeDebug)]
pub struct Message {
    pub kind: String,
    pub timestamp: Timestamp,
    pub key: String, // note key is the same thing as ticker but called key throughout
    pub value: u64,
}

/// Convert a message such as the ascii string "0x123456" into the corresponding bytes
fn eth_hex_decode_helper(message: &[u8]) -> Result<Vec<u8>, OracleError> {
    if !message.starts_with(b"0x") {
        return Err(OracleError::HexParseError);
    }
    hex::decode(&message[2..]).map_err(|_| OracleError::HexParseError)
}

const MAXIMUM_TICKER_LENGTH: usize = 5;

/// Parse an open price feed message. Important note, this function merely parses the message
/// it does not comment on the sanity of the message. All fields should be checked for sanity.
/// The message format is expected to be utf-8 (ascii really) hex characters encoding an ETH ABI
/// binary blob.
///
/// This code is security critical as it is exposed directly to external input. Be extra frosty here.
///
/// Messages and more documentation may be found here
/// https://docs.pro.coinbase.com/#oracle
///
/// Reference implementation is here
/// https://github.com/compound-finance/open-oracle/blob/aff3634c9f23dc40b3803f44863244d22f623e7e/contracts/OpenOraclePriceData.sol#L58
pub fn parse_message(message: &[u8]) -> Result<Message, OracleError> {
    let types = [
        ethabi::param_type::ParamType::String,
        ethabi::param_type::ParamType::Uint(64),
        ethabi::param_type::ParamType::String,
        ethabi::param_type::ParamType::Uint(64),
    ];
    let mut abi_decoded =
        ethabi::decode(&types, &message).map_err(|_| OracleError::HexParseError)?;
    if !abi_decoded.len() == 4 {
        return Err(OracleError::EthAbiParseError);
    }

    let mut abi_drain = abi_decoded.drain(..);

    let kind = abi_drain
        .next()
        .ok_or(OracleError::EthAbiParseError)?
        .to_string()
        .ok_or(OracleError::EthAbiParseError)?;
    if kind != "prices" {
        return Err(OracleError::InvalidKind);
    }

    let timestamp = abi_drain
        .next()
        .ok_or(OracleError::EthAbiParseError)?
        .to_uint()
        .ok_or(OracleError::EthAbiParseError)?
        .as_u64();

    let key = abi_drain
        .next()
        .ok_or(OracleError::EthAbiParseError)?
        .to_string()
        .ok_or(OracleError::EthAbiParseError)?;

    if key.len() > MAXIMUM_TICKER_LENGTH {
        return Err(OracleError::InvalidTicker);
    }

    // todo: it is critical to be aware of overflow during the call to as_u64 but it is not clear to me how to accomplish that
    let value = abi_drain
        .next()
        .ok_or(OracleError::EthAbiParseError)?
        .to_uint()
        .ok_or(OracleError::EthAbiParseError)?
        .as_u64();

    Ok(Message {
        kind,
        timestamp: timestamp * 1000,
        key,
        value,
    })
}

/// The deserialized API response from a given price feed provider.
/// Note that the messages are obviously NOT decoded in this struct.
#[derive(Deserialize)]
pub struct OpenPriceFeedApiResponse {
    pub messages: Vec<String>,
    pub prices: BTreeMap<String, String>,
    pub signatures: Vec<String>,
    pub timestamp: String,
}

/// Parse a JSON message from an API endpoint. See https://docs.pro.coinbase.com/#oracle for
/// message format details.
fn parse_open_price_feed_api_response(
    response: &[u8],
) -> Result<OpenPriceFeedApiResponse, OracleError> {
    serde_json::from_slice(response).map_err(|_| OracleError::JsonParseError)
}

/// Make the open price feed API request to an unauthenticated http endpoint
pub fn open_price_feed_request(url: &str) -> Result<OpenPriceFeedApiResponse, OracleError> {
    let response = open_price_feed_request_unchecked(url)?;

    Ok(response)
}

/// Make the open price feed HTTP API request to an unauthenticated endpoint using HTTP GET.
fn open_price_feed_request_unchecked(url: &str) -> Result<OpenPriceFeedApiResponse, OracleError> {
    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
    let request = http::Request::get(url);
    let pending = request
        .deadline(deadline)
        .send()
        .map_err(|_| OracleError::HttpError)?;
    let response = pending
        .try_wait(deadline)
        .map_err(|_| OracleError::HttpError)?
        .map_err(|_| OracleError::HttpError)?;

    if response.code != 200 {
        return Err(OracleError::HttpError);
    }

    let body = response.body().collect::<Vec<u8>>();
    let parsed = parse_open_price_feed_api_response(&body);

    parsed
}

impl OpenPriceFeedApiResponse {
    /// This is provided for convenience making the processing of API messages as extrinsics
    /// more straightforward.
    pub fn to_message_signature_pairs(
        self,
    ) -> Result<(Vec<(Vec<u8>, Vec<u8>)>, String), OracleError> {
        let mut res = Vec::new();
        // didn't use map here so that we can bail out early using `?` operator
        for (msg, sig) in self.messages.iter().zip(self.signatures) {
            let msg = eth_hex_decode_helper(msg.as_bytes())?;
            let sig = eth_hex_decode_helper(sig.as_bytes())?;
            res.push((msg, sig));
        }

        // XXX possibly introduce a struct here
        Ok((res, self.timestamp))
    }
}

// OCW storage constants
const OCW_LATEST_TIMESTAMP: &[u8; 33] = b"cash::latest_price_feed_timestamp";
const OCW_LATEST_BLOCK_NUMBER: &[u8; 41] = b"cash::latest_price_feed_poll_block_number";
const OCW_STORAGE_LOCK: &[u8; 34] = b"cash::storage_lock_open_price_feed";

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
    let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK);
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
        StorageValueRef::persistent(OCW_LATEST_BLOCK_NUMBER);
    if let Some(Some(latest_poll_block_number)) =
        latest_price_feed_poll_block_number_storage.get::<T::BlockNumber>()
    {
        let poll_interval_blocks =
            <T as frame_system::Config>::BlockNumber::from(ORACLE_POLL_INTERVAL_BLOCKS);
        if block_number - latest_poll_block_number < poll_interval_blocks {
            return Ok(());
        }
    }
    let url = String::from_utf8(url).map_err(|_| OracleError::InvalidApiEndpoint)?;

    // poll
    let (messages_and_signatures, timestamp) =
        open_price_feed_request(&url)?.to_message_signature_pairs()?;

    // Check to see if Coinbase api prices were updated or not
    let latest_price_feed_timestamp_storage = StorageValueRef::persistent(OCW_LATEST_TIMESTAMP);
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

#[cfg(test)]
pub mod tests {
    use super::*;
    use compound_crypto::eth_signature_from_bytes;

    pub static API_RESPONSE_TEST_DATA: &str = r#"
    {
      "messages": [
        "0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000688e4cda00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000",
        "0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000002baa48a00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000",
        "0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000f51180000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034441490000000000000000000000000000000000000000000000000000000000",
        "0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000057e400000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000035a52580000000000000000000000000000000000000000000000000000000000",
        "0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000321900000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034241540000000000000000000000000000000000000000000000000000000000",
        "0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000c63e00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034b4e430000000000000000000000000000000000000000000000000000000000",
        "0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000ad33d80000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000044c494e4b00000000000000000000000000000000000000000000000000000000",
        "0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000009206d00000000000000000000000000000000000000000000000000000000000000000670726963657300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004434f4d5000000000000000000000000000000000000000000000000000000000"
      ],
      "prices": {
        "BTC": "28066.5",
        "ETH": "732.58",
        "DAI": "1.0038",
        "ZRX": "0.36",
        "BAT": "0.2052",
        "KNC": "0.812",
        "LINK": "11.351",
        "COMP": "153.12"
      },
      "signatures": [
        "0x69538bfa1a2097ea206780654d7baac3a17ee57547ee3eeb5d8bcb58a2fcdf401ff8834f4a003193f24224437881276fe76c8e1c0a361081de854457d41d0690000000000000000000000000000000000000000000000000000000000000001c",
        "0x41a3f89a526dee766049f3699e9e975bfbabda4db677c9f5c41fbcc0730fccb84d08b2208c4ffae0b87bb162e2791cc305ee4e9a1d936f9e6154356154e9a8e9000000000000000000000000000000000000000000000000000000000000001c",
        "0x15a9e7019f2b45c5e64646df571ea944b544dbf9093fbe19e41afea68fa58d721e53449245eebea3f351dbdff4dff09cf303a335cb4455f0d3219f308d448483000000000000000000000000000000000000000000000000000000000000001c",
        "0x25be45b4fa82f48160cb0218acafe26e6fea2be797710add737d09ad305ab54e5f75783b857b2c5c526acb3f9b34ffba64c1251843d320f04b5c0efbbe661d17000000000000000000000000000000000000000000000000000000000000001b",
        "0x19984214a69bccb410910de3b277d19fd86f2524510d83b4fc139f1469b11e375297ea89aeda2bceda4a4553e7815f93d3cff192ade88dccf43fb18ba73a97a7000000000000000000000000000000000000000000000000000000000000001b",
        "0x549e608b0e2acc98a36ac88fac610909d430b89c7501183d83c05189260baa6754b16ef74c804f7a7789e4e468878bfe153d76a7029c29f9acce86942a1ff492000000000000000000000000000000000000000000000000000000000000001c",
        "0x01612605d0de98506ced9ca0414a08b7c335cd1dfa0ea2b62d283a2e27d8d33c25eb0abd6cc2625d950f59baf3300a71e269c3f3eea81e5ed8876bb2f4e75cfd000000000000000000000000000000000000000000000000000000000000001b",
        "0x883317a2aa03f1523e95bedb961d7aabfbfba73bb9f54685639d0bc1eb2fd16a7c5510e7f68e1e0824bd5a96093ef921aabb36f79e89defc4d216f6dc0d79fbb000000000000000000000000000000000000000000000000000000000000001b"
      ],
      "timestamp": "1609340760"
    }
    "#;

    #[test]
    fn test_parse_message_happy_path() -> Result<(), OracleError> {
        // note test case taken from https://docs.pro.coinbase.com/#oracle but naturally it may change
        // by the time someone else visits that link
        let test_data = eth_hex_decode_helper("0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005e5da58000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000020f3570580000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000".as_bytes())?;
        let expected = Message {
            kind: "prices".into(),
            timestamp: 1583195520000,
            key: "BTC".into(),
            value: 8845095000,
        };

        let actual = parse_message(&test_data)?;

        assert_eq!(actual, expected);

        Ok(())
    }

    fn get_parsed_test_case() -> OpenPriceFeedApiResponse {
        // test data from OKEX, coinbase pro does not have an unauthenticated endpoint as far as I can tell
        // https://www.okex.com/api/market/v3/oracle
        let actual = parse_open_price_feed_api_response(API_RESPONSE_TEST_DATA.as_bytes()).unwrap();

        actual
    }

    #[test]
    fn test_parse_outer_message_happy_path() {
        let actual = get_parsed_test_case();
        assert_eq!(actual.messages[2], "0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000f51180000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034441490000000000000000000000000000000000000000000000000000000000");
        assert_eq!(actual.prices["BAT"], "0.2052");
        assert_eq!(actual.signatures[2], "0x15a9e7019f2b45c5e64646df571ea944b544dbf9093fbe19e41afea68fa58d721e53449245eebea3f351dbdff4dff09cf303a335cb4455f0d3219f308d448483000000000000000000000000000000000000000000000000000000000000001c");
        assert_eq!(actual.timestamp, "1609340760");
    }

    #[test]
    fn test_recover() {
        let msg = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000688e4cda00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000");
        let sig = hex_literal::hex!("69538bfa1a2097ea206780654d7baac3a17ee57547ee3eeb5d8bcb58a2fcdf401ff8834f4a003193f24224437881276fe76c8e1c0a361081de854457d41d0690000000000000000000000000000000000000000000000000000000000000001c");
        let hashed = compound_crypto::keccak(&msg);
        let recovered =
            compound_crypto::eth_recover(&hashed, &eth_signature_from_bytes(&sig).unwrap(), true)
                .unwrap();
        assert_eq!(
            hex::encode(recovered),
            "85615b076615317c80f14cbad6501eec031cd51c"
        )
    }
}
