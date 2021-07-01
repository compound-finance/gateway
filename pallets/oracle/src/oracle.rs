use frame_support::storage::{StorageMap, StorageValue};
use serde::Deserialize;
use sp_runtime::offchain::{
    http,
    storage::StorageValueRef,
    storage_lock::{StorageLock, Time},
    Duration,
};

use crate::{
    error::OracleError,
    ticker::Ticker,
    types::{AssetPrice, Timestamp},
};
use crate::{Config, PriceReporters, PriceTimes, Prices, ORACLE_POLL_INTERVAL_BLOCKS};
use our_std::convert::TryInto;
use our_std::{collections::btree_map::BTreeMap, str::FromStr, vec::Vec, RuntimeDebug};
use timestamp::GetConvertedTimestamp;

pub const MAX_PRICE_FUTURE_MS: Timestamp = 100000u64; //100 seconds

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
const ORACLE_FETCH_DEADLINE: u64 = 2_000;

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
        Err(OracleError::EthAbiParseError)?;
    }

    let mut abi_drain = abi_decoded.drain(..);

    let kind = abi_drain
        .next()
        .ok_or(OracleError::EthAbiParseError)?
        .to_string()
        .ok_or(OracleError::EthAbiParseError)?;
    if kind != "prices" {
        Err(OracleError::InvalidKind)?;
    }

    let timestamp: u64 = abi_drain
        .next()
        .ok_or(OracleError::EthAbiParseError)?
        .to_uint()
        .ok_or(OracleError::EthAbiParseError)?
        .try_into()
        .map_err(|_| OracleError::InvalidTimestamp)?;

    let key = abi_drain
        .next()
        .ok_or(OracleError::EthAbiParseError)?
        .to_string()
        .ok_or(OracleError::EthAbiParseError)?;

    if key.len() > MAXIMUM_TICKER_LENGTH {
        Err(OracleError::InvalidTicker)?;
    }

    let value: u64 = abi_drain
        .next()
        .ok_or(OracleError::EthAbiParseError)?
        .to_uint()
        .ok_or(OracleError::EthAbiParseError)?
        .try_into()
        .map_err(|_| OracleError::InvalidValue)?;

    Ok(Message {
        kind,
        timestamp: timestamp
            .checked_mul(1000)
            .ok_or(OracleError::InvalidTimestamp)?,
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
    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(ORACLE_FETCH_DEADLINE));
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
    pub fn to_message_signature_pairs(self) -> Result<(Vec<(Vec<u8>, Vec<u8>)>, u64), OracleError> {
        let mut pairs = Vec::new();
        // didn't use map here so that we can bail out early using `?` operator
        for (msg, sig) in self.messages.iter().zip(self.signatures) {
            let msg = eth_hex_decode_helper(msg.as_bytes())?;
            let sig = eth_hex_decode_helper(sig.as_bytes())?;
            pairs.push((msg, sig));
        }
        let ts = self
            .timestamp
            .parse::<u64>()
            .map_err(|_| OracleError::InvalidTimestamp)?;

        Ok((pairs, ts))
    }
}

// OCW storage constants
const OCW_LATEST_BLOCK_NUMBER: &[u8; 41] = b"cash::latest_price_feed_poll_block_number";
const OCW_STORAGE_LOCK: &[u8; 34] = b"cash::storage_lock_open_price_feed";

pub fn check_signature<T: Config>(
    payload: &Vec<u8>,
    signature: &Vec<u8>,
) -> Result<bool, OracleError> {
    // check signature
    let parsed_sig: [u8; 65] = gateway_crypto::eth_signature_from_bytes(&signature)?;

    // note that this is actually a double-hash situation but that is expected behavior
    // the hashed message is hashed again in the eth convention inside eth_recover
    let hashed = gateway_crypto::keccak(&payload);
    let recovered =
        runtime_interfaces::keyring_interface::eth_recover(hashed.into(), parsed_sig, true)?;

    Ok(PriceReporters::get().contains(recovered))
}

pub fn get_and_check_parsed_price<T: Config>(
    payload: &Vec<u8>,
) -> Result<(Message, Ticker), OracleError> {
    // parse message and check it
    let parsed = parse_message(payload)?;
    let ticker = Ticker::from_str(&parsed.key)?;

    let current_timestamp = T::GetConvertedTimestamp::get_recent_timestamp()
        .map_err(|_| OracleError::InvalidTimestamp)?;
    let time_to_parsed_price = parsed
        .timestamp
        .checked_sub(current_timestamp)
        .ok_or(OracleError::InvalidTimestamp)?;
    if let Some(last_updated) = PriceTimes::get(&ticker) {
        if parsed.timestamp <= last_updated {
            Err(OracleError::StalePrice)?;
        } else if time_to_parsed_price > MAX_PRICE_FUTURE_MS {
            Err(OracleError::TimestampTooHigh)?;
        }
    }
    Ok((parsed, ticker))
}
pub fn post_price<T: Config>(payload: Vec<u8>, signature: Vec<u8>) -> Result<(), OracleError> {
    if !check_signature::<T>(&payload, &signature)? {
        Err(OracleError::InvalidReporter)?;
    }

    let (parsed, ticker) = get_and_check_parsed_price::<T>(&payload)?;

    // * WARNING begin storage - all checks must happen above * //

    Prices::insert(&ticker, parsed.value as AssetPrice);
    PriceTimes::insert(&ticker, parsed.timestamp as Timestamp);
    Ok(())
}

/// Procedure for offchain worker to processes messages coming out of the open price feed
pub fn process_prices<T: Config>(block_number: T::BlockNumber) -> Result<(), OracleError> {
    let mut lock = StorageLock::<Time>::new(OCW_STORAGE_LOCK);
    if lock.try_lock().is_err() {
        // working in another thread, no big deal
        return Ok(());
    }

    // get the URL to poll, just return if there is no URL set up
    let url =
        runtime_interfaces::validator_config_interface::get_opf_url().unwrap_or(String::new());
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

    // poll
    let (messages_and_signatures, timestamp) =
        open_price_feed_request(&url)?.to_message_signature_pairs()?;

    let curr_ts = runtime_interfaces::price_feed_interface::get_price_data_ts();
    if curr_ts.map(|v| v < timestamp).unwrap_or(true) {
        runtime_interfaces::price_feed_interface::set_price_data(
            messages_and_signatures,
            timestamp,
        );
    }

    latest_price_feed_poll_block_number_storage.set(&block_number);
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::tests::*;
    use gateway_crypto::eth_signature_from_bytes;

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

    #[test]
    fn test_check_price_future_timestamp() {
        new_test_ext().execute_with(|| {
            let ticker = Ticker::new("ETH");
            let start_timestamp = 2u64;

            let kind = ethabi::Token::String(String::from("prices"));
            // posting a price from too far in the future
            let timestamp = ethabi::Token::Uint((MAX_PRICE_FUTURE_MS + start_timestamp + 1).into());
            let key = ethabi::Token::String(String::from("ETH"));
            let value = ethabi::Token::Uint(100u64.into());

            let v = ethabi::encode(&vec![kind, timestamp, key, value]);

            <pallet_timestamp::Pallet<Test>>::set_timestamp(start_timestamp);
            PriceTimes::insert(ticker, start_timestamp);

            assert_eq!(
                get_and_check_parsed_price::<Test>(&v),
                Err(OracleError::TimestampTooHigh)
            );
        });
    }

    #[test]
    fn test_check_price_happy_path() {
        new_test_ext().execute_with(|| {
            let ticker = Ticker::new("ETH");
            PriceTimes::insert(ticker, 0);

            let kind = ethabi::Token::String(String::from("prices"));
            let timestamp = ethabi::Token::Uint((1).into());
            let key = ethabi::Token::String(String::from("ETH"));
            let value = ethabi::Token::Uint(100u64.into());

            let v = ethabi::encode(&vec![kind, timestamp, key, value]);
            <pallet_timestamp::Pallet<Test>>::set_timestamp(500);
            assert_ok!(get_and_check_parsed_price::<Test>(&v));
        });
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
    fn test_parse_message_overflowing_timestamp() -> Result<(), OracleError> {
        // This input contains a timestamp value of 1152921506190042496. This test checks that the
        // `parse_message` won't crash on a `timestamp: timestamp * 1000,` calculation and will
        // instead return an error of InvalidTimestamp (note that before this line panicked due
        // to overflow in debug builds)
        let v = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16,
            0, 0, 0, 94, 93, 165, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 15, 53, 112, 88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 112, 114, 105, 99, 101,
            115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 3, 66, 84, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let actual = parse_message(&v);
        assert_eq!(actual, Err(OracleError::InvalidTimestamp));
        Ok(())
    }

    #[test]
    fn test_parse_message_timestamp_exceeds_u64() -> Result<(), OracleError> {
        // This panics as the decoded value exceeds U64 type.
        // This is because the values are first decoded to U256 and then moved to U64
        //
        // Related sources:
        // - definition of U256 - https://github.com/paritytech/parity-common/blob/primitive-types-v0.7.3/primitive-types/src/lib.rs#L38-L42
        // - conversion to U64 with overflow checking - https://github.com/paritytech/parity-common/blob/uint-v0.8.5/uint/src/uint.rs#L519-L531
        //
        // So the panic is triggered on `.as_u64()` because the decoder decodes U256 and then converts it to U64 panicking on overflows.
        let v = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 94, 93, 165, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 15, 53, 112, 88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 112, 114, 105, 99, 101,
            115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 3, 66, 84, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let actual = parse_message(&v);
        assert_eq!(actual, Err(OracleError::InvalidTimestamp)); // TODO/FIXME: add OracleError::InvalidTimestampValue or other?
        Ok(())
    }

    #[test]
    fn test_parse_message_timestamp_exceeds_u64_2() -> Result<(), OracleError> {
        // This is the same as `test_parse_message_timestamp_exceeds_u64` but here, the overflow happens with `value`.
        let v = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 128, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 94, 93, 165, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 15, 53, 112, 88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 112, 114, 105, 99, 101,
            115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 66, 84, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let actual = parse_message(&v);
        assert_eq!(actual, Err(OracleError::InvalidTimestamp)); // TODO/FIXME: add OracleError::InvalidTimestampValue or other?
        Ok(())
    }

    #[test]
    fn test_parse_message_timestamp_exceeds_u64_3() -> Result<(), OracleError> {
        // This is the same as `test_parse_message_timestamp_exceeds_u64` but here, the overflow happens with `value`.
        let v = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 128, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 94, 93, 165, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 15, 53, 112, 88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 112, 114, 105, 99, 101,
            115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 3, 66, 84, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let actual = parse_message(&v);
        assert_eq!(actual, Err(OracleError::InvalidTimestamp)); // TODO/FIXME: add OracleError::InvalidTimestampValue or other?
        Ok(())
    }

    #[test]
    fn test_parse_message_timestamp_exceeds_u64_4() -> Result<(), OracleError> {
        // This is the same as `test_parse_message_timestamp_exceeds_u64` but here, the overflow happens with `value`.
        let v = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 128, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 94, 93, 165, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 15, 53, 112, 88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 112, 114, 105, 99, 101,
            115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 3, 66, 84, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let actual = parse_message(&v);
        assert_eq!(actual, Err(OracleError::InvalidTimestamp)); // TODO/FIXME: add OracleError::InvalidTimestampValue or other?
        Ok(())
    }

    #[test]
    fn test_parse_message_timestamp_exceeds_u64_5() -> Result<(), OracleError> {
        // This is the same as `test_parse_message_timestamp_exceeds_u64` but here, the overflow happens with `value`.
        let v = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 94, 93, 165, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 15, 53, 112, 88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 112, 114, 105, 99, 101,
            115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 3, 66, 84, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let actual = parse_message(&v);
        assert_eq!(actual, Err(OracleError::InvalidTimestamp)); // TODO/FIXME: add OracleError::InvalidTimestampValue or other?
        Ok(())
    }

    #[test]
    fn test_parse_message_timestamp_exceeds_u64_6() -> Result<(), OracleError> {
        // This is the same as `test_parse_message_timestamp_exceeds_u64` but here, the overflow happens with `value`.
        let v = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 128, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 94, 93, 165, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 15, 53, 112, 88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 112, 114, 105, 99, 101,
            115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 3, 66, 84, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let actual = parse_message(&v);
        assert_eq!(actual, Err(OracleError::InvalidTimestamp)); // TODO/FIXME: add OracleError::InvalidTimestampValue or other?
        Ok(())
    }
    #[test]
    fn test_parse_message_timestamp_exceeds_u64_7() -> Result<(), OracleError> {
        // This is the same as `test_parse_message_timestamp_exceeds_u64` but here, the overflow happens with `value`.
        let v = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 128, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 94, 93, 165, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 15, 53, 112, 88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 112, 114, 105, 99, 101,
            115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 15, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 66, 84, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let actual = parse_message(&v);
        assert_eq!(actual, Err(OracleError::InvalidTimestamp)); // TODO/FIXME: add OracleError::InvalidTimestampValue or other?
        Ok(())
    }

    #[test]
    fn test_parse_message_timestamp_exceeds_u64_8() -> Result<(), OracleError> {
        // This is the same as `test_parse_message_timestamp_exceeds_u64` but here, the overflow happens with `value`.
        let v = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 94, 93, 165, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 15, 53, 112, 88, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 112, 114, 105, 99, 101,
            115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 3, 66, 84, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let actual = parse_message(&v);
        assert_eq!(actual, Err(OracleError::InvalidTimestamp)); // TODO/FIXME: add OracleError::InvalidTimestampValue or other?
        Ok(())
    }

    // TODO/FIXME: Add a testcase for value exceeding u64 in parse_message

    #[test]
    fn test_recover() {
        let msg = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000688e4cda00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000");
        let sig = hex_literal::hex!("69538bfa1a2097ea206780654d7baac3a17ee57547ee3eeb5d8bcb58a2fcdf401ff8834f4a003193f24224437881276fe76c8e1c0a361081de854457d41d0690000000000000000000000000000000000000000000000000000000000000001c");
        let hashed = gateway_crypto::keccak(&msg);
        let recovered =
            gateway_crypto::eth_recover(&hashed, &eth_signature_from_bytes(&sig).unwrap(), true)
                .unwrap();
        assert_eq!(
            hex::encode(recovered),
            "85615b076615317c80f14cbad6501eec031cd51c"
        )
    }
}
