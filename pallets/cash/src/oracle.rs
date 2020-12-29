use crate::Timestamp;
use our_std::Debuggable;

#[derive(Debuggable)]
pub enum OracleError {
    HexParseError,
    EthAbiParseError,
    InvalidKind,
    InvalidTicker,
}

#[derive(PartialEq, Eq, Debuggable)]
pub struct Message {
    pub kind: String,
    pub timestamp: Timestamp,
    pub key: String, // note key is the same thing as ticker but called key throughout
    pub value: u64,
}

const MAXIMUM_TICKER_LENGTH: usize = 5;

/// Parse an open price feed message. Important note, this function merely parses the message
/// it does not comment on the sanity of the message. All fields should be checked for sanity.
///
/// This code is security critical as it is exposed directly to external input. Be extra frosty here.
///
/// Messages and more documentation may be found here
/// https://docs.pro.coinbase.com/#oracle
///
/// Reference implementation is here
/// https://github.com/compound-finance/open-oracle/blob/aff3634c9f23dc40b3803f44863244d22f623e7e/contracts/OpenOraclePriceData.sol#L58
fn parse_message(message: &str) -> Result<Message, OracleError> {
    let types = [
        ethabi::param_type::ParamType::String,
        ethabi::param_type::ParamType::Uint(64),
        ethabi::param_type::ParamType::String,
        ethabi::param_type::ParamType::Uint(64),
    ];
    if !message.starts_with("0x") {
        return Err(OracleError::HexParseError);
    }
    // strip 0x
    let hex_decoded = hex::decode(&message[2..]).map_err(|_| OracleError::HexParseError)?;
    let mut abi_decoded =
        ethabi::decode(&types, &hex_decoded).map_err(|_| OracleError::HexParseError)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message_happy_path() -> Result<(), OracleError> {
        // note test case taken from https://docs.pro.coinbase.com/#oracle but naturally it may change
        // by the time someone else visits that link
        let test_data = "0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005e5da58000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000020f3570580000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000";
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
}
