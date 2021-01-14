pub fn hex_to_u128(hex: &[u8]) -> Option<u128> {
    let len = hex.len();
    if len > 16 {
        return None;
    } else {
        return Some(
            hex.iter()
                .fold((len as u32, 0u128), |(pos, sum), el| {
                    (pos - 1, sum + (*el as u128) * 256u128.pow(pos - 1))
                })
                .1,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_hex() {
        // Empty
        assert_eq!(hex_to_u128(&[]), Some(0));
        // 1 byte
        assert_eq!(hex_to_u128(&[55]), Some(55));
        // 2 bytes
        assert_eq!(hex_to_u128(&[1, 0]), Some(256));
        // 16 bytes
        assert_eq!(
            hex_to_u128(&[
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
            ]),
            Some(340282366920938463463374607431768211455)
        );
        // 17 bytes
        assert_eq!(
            hex_to_u128(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            None
        );
    }
}
