use crate::Timestamp;
use our_std::{collections::btree_map::BTreeMap, vec::Vec, Debuggable};
use serde::Deserialize;
use sp_runtime::offchain::{http, Duration};

/// Errors coming from the price oracle
#[derive(Debuggable)]
pub enum OracleError {
    HexParseError,
    EthAbiParseError,
    InvalidKind,
    InvalidTicker,
    JsonParseError,
    HttpError,
    InvalidOpenOracleApiResponse,
    InvalidSignature,
}

/// A single decoded message from the price oracle
#[derive(PartialEq, Eq, Debuggable)]
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
        .as_u128();

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
        timestamp,
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

pub const OKEX_OPEN_PRICE_FEED_URL: &str = "https://www.okex.com/api/market/v3/oracle";

/// Make the open price feed API request to OKEx.
pub fn open_price_feed_request_okex() -> Result<OpenPriceFeedApiResponse, OracleError> {
    let response = open_price_feed_request_unauthenticated(OKEX_OPEN_PRICE_FEED_URL)?;
    sanity_check_messages(&response)?;

    Ok(response)
}

/// Make the open price feed HTTP API request to an unauthenticated endpoint using HTTP GET.
fn open_price_feed_request_unauthenticated(
    url: &str,
) -> Result<OpenPriceFeedApiResponse, OracleError> {
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

/// Sanity checks on the entire API response payload. Not sure how far we want to go here in terms of
/// using useful data when it is present. There is a balance between using useful messages that
/// have good data but the payload is somewhat inconsistent vs not being able to post new prices
/// due to an inconsistency. The distributed nature of the open price feed should help us here.
fn sanity_check_messages(api_response: &OpenPriceFeedApiResponse) -> Result<(), OracleError> {
    if api_response.messages.len() != api_response.signatures.len() {
        return Err(OracleError::InvalidOpenOracleApiResponse);
    }
    let timestamp: Timestamp = api_response
        .timestamp
        .parse()
        .map_err(|_| OracleError::InvalidOpenOracleApiResponse)?;

    // decode messages and check content
    for message in &api_response.messages {
        let message = eth_hex_decode_helper(message.as_bytes())?;
        let decoded_message = parse_message(&message)?;

        if decoded_message.timestamp != timestamp {
            return Err(OracleError::InvalidOpenOracleApiResponse);
        }

        if !api_response.prices.contains_key(&decoded_message.key) {
            return Err(OracleError::InvalidOpenOracleApiResponse);
        }

        let payload_price = api_response
            .prices
            .get(&decoded_message.key)
            .ok_or(OracleError::InvalidOpenOracleApiResponse)?;
        let price_int = payload_price.replace(".", "");
        let price_int = price_int.trim_start_matches("0");
        let message_price = format!("{}", decoded_message.value);
        let message_price = message_price.trim_end_matches("0");
        // this check is not very good because the value could still be off by an order of magnitude
        // but without the associated decimals this is what we can do
        if price_int != message_price {
            return Err(OracleError::InvalidOpenOracleApiResponse);
        }
    }

    Ok(())
}

impl OpenPriceFeedApiResponse {
    /// This is provided for convenience making the processing of API messages as extrinsics
    /// more straightforward.
    pub fn to_message_signature_pairs(self) -> Result<Vec<(Vec<u8>, Vec<u8>)>, OracleError> {
        let mut res = Vec::new();
        // didn't use map here so that we can bail out early using `?` operator
        for (msg, sig) in self.messages.iter().zip(self.signatures) {
            let msg = eth_hex_decode_helper(msg.as_bytes())?;
            let sig = eth_hex_decode_helper(sig.as_bytes())?;
            res.push((msg, sig));
        }

        Ok(res)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

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
            timestamp: 1583195520,
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
    fn test_sanity_check_messages_happy_path() {
        let actual = get_parsed_test_case();
        sanity_check_messages(&actual).unwrap();
    }

    #[test]
    fn test_recover() {
        let msg = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000688e4cda00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000");
        let sig = hex_literal::hex!("69538bfa1a2097ea206780654d7baac3a17ee57547ee3eeb5d8bcb58a2fcdf401ff8834f4a003193f24224437881276fe76c8e1c0a361081de854457d41d0690000000000000000000000000000000000000000000000000000000000000001c");
        let hashed = compound_crypto::keccak(&msg);
        let recovered = compound_crypto::eth_recover(&hashed, &sig).unwrap();
        assert_eq!(
            hex::encode(recovered),
            "85615b076615317c80f14cbad6501eec031cd51c"
        )
    }
}
