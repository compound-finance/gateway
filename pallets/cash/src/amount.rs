use anyhow::{bail, Error, Result};
use codec::{Decode, Encode, Input};
use num_bigint::BigUint;

/// The type of the decimal field.
pub type DecimalType = u8;

/// The type of the mantissa field
pub type MantissaType = BigUint;

/// The type for Cash
pub type CashAmount = u128;

/// Represents a decimal number in base 10 with fixed precision. The number of decimals depends
/// on the amount being represented and is not stored alongside the amount.
///
/// For example, if the mantissa is 123456789 and decimals is 4 the number that is represented is
/// 12345.6789. The decimals are stored separately.
#[derive(Clone, PartialEq, Debug)]
pub struct Amount {
    pub mantissa: MantissaType,
    pub decimals: DecimalType,
}

impl Encode for Amount {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        let mut mantissa_bytes = self.mantissa.to_bytes_le();
        mantissa_bytes.push(self.decimals);
        mantissa_bytes.using_encoded(f)
    }
}

impl Decode for Amount {
    fn decode<I: Input>(value: &mut I) -> Result<Self, codec::Error> {
        let mut value_bytes: Vec<u8> = Decode::decode(value)?;
        let decimals: DecimalType = value_bytes.remove(value_bytes.len() - 1);
        let mantissa_le_encoded = value_bytes;
        let amount = Amount {
            mantissa: BigUint::from_bytes_le(&mantissa_le_encoded),
            decimals: decimals,
        };

        Ok(amount)
    }
}

/// Error type for fixed precision math.
/// todo: is this necessary now?
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MathError {
    PrecisionMismatch,
}

impl Amount {
    /// Create a new FixedPrecision number from parts. The mantissa is used "raw" and not scaled
    /// in any way
    pub fn new<T: Into<MantissaType>, D: Into<DecimalType>>(mantissa: T, decimals: D) -> Self {
        Amount {
            mantissa: mantissa.into(),
            decimals: decimals.into(),
        }
    }

    /// Add two FixedPrecision numbers together. Note the signature uses borrowed values this is
    /// because the underlying storage is arbitrarily large and we do not want to copy the values.
    pub fn add(self: &Self, rhs: &Self) -> Result<Self> {
        if self.decimals != rhs.decimals {
            bail!(
                "Mismatched decimals for amounts: {} vs {}",
                self.decimals,
                rhs.decimals
            );
        }

        // note - this cannot fail with BigUint but that will change based on underlying storage
        let new_mantissa = &self.mantissa + &rhs.mantissa;

        Ok(Self::new(new_mantissa, self.decimals))
    }

    /// Create the representation of 1 in the number of decimals requested. For example one(3)
    /// will return a fixed precision number with 1000 as the mantissa and 3 as the number of decimals
    pub fn one<T: Into<DecimalType> + Copy>(decimals: T) -> Self {
        let ten: MantissaType = 10u8.into();
        let decimals_cast: DecimalType = decimals.into();
        let new_mantissa = ten.pow(decimals_cast as u32);
        Self::new(new_mantissa, decimals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one() {
        let expected = Amount::new(1000u32, 3);
        let actual = Amount::one(3);
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_add_happy_path() {
        let a = Amount::one(2);
        let b = Amount::one(2);
        // note - automatic borrow of `a` here (rust elides the (&a).add for you
        let actual = a.add(&b).unwrap();

        // make sure nothing has changed
        assert_eq!(a, b);
        assert_eq!(a, Amount::one(2));
        assert_eq!(b, Amount::one(2));

        let expected = Amount::new(200u8, 2);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_add_error() {
        let a = Amount::one(2);
        let b = Amount::new(2000_u32, 3);

        assert_eq!(
            a.add(&b).unwrap_err().to_string(),
            "Mismatched decimals for amounts: 2 vs 3"
        );
    }

    #[test]
    fn test_scale_codec() -> Result<(), codec::Error> {
        let expected = Amount::new(6000u32, 3);
        let encoded = expected.encode();
        let actual: Amount = Decode::decode(&mut encoded.as_slice())?;
        assert_eq!(expected, actual);

        Ok(())
    }
}
