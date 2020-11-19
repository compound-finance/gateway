use primitive_types::U256;

macro_rules! impl_fixed_precision_number {
    ($t:ident, $name:ident) => {
        #[derive(Copy, Clone, Eq, PartialEq, Hash)]
        pub struct $name {
            decimals: usize,
            value: $t,
        }

        pub fn new_fixed_precision_number(decimals: usize, value: $t) -> $name {
            return $name { decimals, value };
        }

        impl $name {
            pub fn overflowing_add(self, other: $name) -> ($name, bool) {
                // todo: check decimals match
                if self.decimals != other.decimals {
                    return (self, true);
                }
                let (new_value, overflowed) = self.value.overflowing_add(other.value);
                let result = new_fixed_precision_number(self.decimals, new_value);
                (result, overflowed)
            }
        }
    };
}

impl_fixed_precision_number!(U256, FixedPrecisionNumberU256);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let x = new_fixed_precision_number(2, U256::one());
        let (y, overflowed) = x.overflowing_add(x);
        assert_eq!(overflowed, false);
        let expected = U256::from(2);
        assert_eq!(y.value, expected);
    }
}
