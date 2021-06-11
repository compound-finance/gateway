use our_std::convert::TryInto;

pub fn decode_hex(data: &String) -> Option<Vec<u8>> {
    if data.len() < 2 || !data.starts_with("0x") {
        None
    } else {
        hex::decode(&data[2..]).ok()
    }
}

pub fn decode_topic(topic: &String) -> Option<ethabi::Hash> {
    let res = decode_hex(topic)?;
    let addr: &[u8; 32] = &res[..].try_into().ok()?;
    Some(addr.into())
}

pub fn parse_word(val_opt: Option<String>) -> Option<[u8; 32]> {
    let v = decode_hex(&val_opt?)?;
    let tokens = ethabi::decode(&[ethabi::ParamType::FixedBytes(32)], &v[..]).ok()?;
    match &tokens[..] {
        [ethabi::token::Token::FixedBytes(bytes)] => bytes[..].try_into().ok(),
        _ => None,
    }
}

// Note: our hex library won't even _parse_ hex with an odd-number of digits
//       so we need to pad before we parse with ethabi, as opposed to decoding
//       and then padding.
// Note: this is an internal function and does not parse the hex digits themselves
fn pad(val: String) -> Option<String> {
    if val.len() > 66 || val.len() < 2 || !val.starts_with("0x") {
        None
    } else {
        let mut s = String::with_capacity(64);
        let padding = 66 - val.len();
        for _ in 0..padding {
            s.push('0');
        }
        s.push_str(&val[2..]);
        Some(s)
    }
}

pub fn parse_u64(val_opt: Option<String>) -> Option<u64> {
    let padded = hex::decode(&pad(val_opt?)?).ok()?;
    let tokens = ethabi::decode(&[ethabi::ParamType::Uint(256)], &padded[..]).ok()?;
    match tokens[..] {
        [ethabi::token::Token::Uint(uint)] => uint.try_into().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_hex() {
        assert_eq!(decode_hex(&String::from("0x")), Some(vec![]));
        assert_eq!(decode_hex(&String::from("0x01")), Some(vec![1]));
        assert_eq!(decode_hex(&String::from("0x0102")), Some(vec![1, 2]));
        assert_eq!(decode_hex(&String::from("0x3")), None);
        assert_eq!(decode_hex(&String::from("0b01")), None);
        assert_eq!(decode_hex(&String::from("")), None);
        assert_eq!(decode_hex(&String::from("0")), None);
        assert_eq!(decode_hex(&String::from("0xr")), None);
        assert_eq!(decode_hex(&String::from("0💙")), None);
        assert_eq!(decode_hex(&String::from("0x💙")), None);
        assert_eq!(decode_hex(&String::from("0x💙💙")), None);
        assert_eq!(decode_hex(&String::from("0xṰ̺̺̕o͞ ̷i̲̬͇̪͙n̝̗͕v̟̜̘̦͟o̶̙̰̠k")), None);
    }

    #[test]
    fn test_decode_topic() {
        assert_eq!(decode_topic(&String::from("0x01")), None);
        assert_eq!(
            decode_topic(&String::from(
                "0x000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f"
            ))
            .unwrap()
            .to_fixed_bytes(),
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
                10, 11, 12, 13, 14, 15
            ]
        );

        assert_eq!(
            decode_topic(&String::from(
                "0x000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f00"
            )),
            None
        );
    }

    #[test]
    fn test_parse_word() {
        assert_eq!(parse_word(None), None);
        assert_eq!(parse_word(Some(String::from("0x01"))), None);
        assert_eq!(
            parse_word(Some(String::from(
                "0x000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f"
            ))),
            Some([
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
                10, 11, 12, 13, 14, 15
            ])
        );

        assert_eq!(
            parse_word(Some(String::from(
                "0x000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f00"
            ))),
            None
        );
    }

    #[test]
    fn test_pad() {
        assert_eq!(pad(String::from("")), None);
        assert_eq!(pad(String::from("0")), None);
        assert_eq!(
            pad(String::from("0x")),
            Some(String::from(
                "0000000000000000000000000000000000000000000000000000000000000000"
            ))
        );
        assert_eq!(
            pad(String::from("0x1")),
            Some(String::from(
                "0000000000000000000000000000000000000000000000000000000000000001"
            ))
        );
        // Note: we do not parse the data at all, so this is valid
        assert_eq!(
            pad(String::from("0xr")),
            Some(String::from(
                "000000000000000000000000000000000000000000000000000000000000000r"
            ))
        );
        assert_eq!(
            pad(String::from("0x11")),
            Some(String::from(
                "0000000000000000000000000000000000000000000000000000000000000011"
            ))
        );
        assert_eq!(
            pad(String::from("0x111")),
            Some(String::from(
                "0000000000000000000000000000000000000000000000000000000000000111"
            ))
        );
        assert_eq!(
            pad(String::from(
                "0x1111111111111111111111111111111111111111111111111111111111111111"
            )),
            Some(String::from(
                "1111111111111111111111111111111111111111111111111111111111111111"
            ))
        );
        assert_eq!(
            pad(String::from(
                "0x001111111111111111111111111111111111111111111111111111111111111111"
            )),
            None
        );
    }

    #[test]
    fn test_parse_u64() {
        assert_eq!(parse_u64(Some(String::from(""))), None);
        assert_eq!(parse_u64(Some(String::from("0"))), None);
        assert_eq!(parse_u64(Some(String::from("0x"))), Some(0u64));
        assert_eq!(parse_u64(Some(String::from("0x0"))), Some(0u64));
        assert_eq!(parse_u64(Some(String::from("0x1"))), Some(1u64));
        assert_eq!(parse_u64(Some(String::from("0x11"))), Some(17u64));
        assert_eq!(parse_u64(Some(String::from("0x111"))), Some(273u64));
        assert_eq!(
            parse_u64(Some(String::from("0xffffffffffffffff"))),
            Some(0xffffffffffffffff)
        );
        assert_eq!(parse_u64(Some(String::from("0x11ffffffffffffffff"))), None);
    }
}
