/// A helper function for from_nominal on Price.
///
/// Only for use in const contexts.
pub const fn uint_from_string_with_decimals(decimals: u8, s: &'static str) -> u128 {
    int_from_string_with_decimals(decimals, s) as u128
}

/// Only for use in const contexts.
pub const fn int_from_string_with_decimals(decimals: u8, s: &'static str) -> i128 {
    let bytes = s.as_bytes();
    let mut i = bytes.len();
    let mut provided_fractional_digits = 0;
    let mut past_decimal = false;
    let mut tenpow: i128 = 1;
    let mut qty: i128 = 0;

    // note - for loop is not allowed in `const` context
    // going from the right of the string
    loop {
        i -= 1;
        let byte = bytes[i];
        if byte == b'-' {
            if i != 0 {
                // quit, a dash somewhere it should not be
                let _should_overflow = byte + u8::max_value();
            }
            // negate
            qty *= -1;
            break;
        }

        if byte == b'.' {
            if past_decimal {
                // multiple radix - quit.
                let _should_overflow = byte + u8::max_value();
            }
            past_decimal = true;
            continue;
        }

        if !past_decimal {
            provided_fractional_digits += 1;
        }
        // will underflow whenever byte < b'0'
        let byte_as_num = byte - b'0';
        // will overflow whenever byte > b'9'
        let _should_overflow = byte + (u8::max_value() - b'9');

        qty += (byte_as_num as i128) * tenpow;
        tenpow *= 10;
        if i == 0 {
            break;
        }
    }

    if bytes.len() == 1 && past_decimal {
        // only a radix provided, quit
        let _should_overflow = bytes[0] + u8::max_value();
    }

    // never passed the radix, it is a whole number
    if !past_decimal {
        provided_fractional_digits = 0;
    }

    let number_of_zeros_to_scale_up = decimals - provided_fractional_digits;
    if number_of_zeros_to_scale_up == 0 {
        return qty;
    }

    let scalar = static_pow10(number_of_zeros_to_scale_up) as i128;
    qty * scalar
}

/// Statically get the u128 corresponding to some number of decimals.
pub const fn static_pow10(decimals: u8) -> u128 {
    let mut v: u128 = 1;
    let mut i = 0;
    loop {
        if i >= decimals {
            return v;
        }
        i += 1;
        v *= 10;
    }
}
