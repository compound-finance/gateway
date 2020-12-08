use anyhow::{bail, Error, Result};
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
    mantissa: MantissaType,
    decimals: DecimalType,
}

/// Error type for fixed precision math.
/// todo: is this necessary now?
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MathError {
    PrecisionMismatch,
    InsufficientPrecision,
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

    /// A helper function to check that the decimals on two Amounts match
    /// A PrecisionMismatch MathError is returned whenever they do not match.
    fn check_decimals(self: &Self, rhs: &Self) -> Result<(), MathError> {
        if self.decimals != rhs.decimals {
            return Err(MathError::PrecisionMismatch);
        }

        Ok(())
    }

    /// Add two FixedPrecision numbers together. Note the signature uses borrowed values this is
    /// because the underlying storage is arbitrarily large and we do not want to copy the values.
    pub fn add(self: &Self, rhs: &Self) -> Result<Self, MathError> {
        self.check_decimals(rhs)?;
        // note - this cannot fail with BigUint but that will change based on underlying storage
        let new_mantissa = &self.mantissa + &rhs.mantissa;

        Ok(Self::new(new_mantissa, self.decimals))
    }

    /// Multiply two numbers with the same number of decimals of precision
    /// Example 2 times 2 with 4 digits of precision -> mantissa 40000 digits 4
    pub fn mul(self: &Self, rhs: &Self) -> Result<Self, MathError> {
        self.check_decimals(rhs)?;

        let one = Self::one(self.decimals);

        // need to divide by the mantissa of one to properly scale
        // todo: is there a better way to do this without having to compute so many useless digits?
        let new_mantissa = self.clone().mantissa * rhs.clone().mantissa / one.mantissa;

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

    /// Return the number 0.5 with a given number of decimals
    /// For example, half(3) will give an amount with a mantissa of 500 and decimals of 3
    pub fn half<T: Into<DecimalType> + Copy>(decimals: T) -> Result<Self, MathError> {
        let decimals: DecimalType = decimals.into();
        if decimals < 1 {
            return Err(MathError::InsufficientPrecision);
        }
        let one = Self::one(decimals - 1);
        let new_mantissa: MantissaType = one.mantissa * 5u8;

        Ok(Self::new(new_mantissa, decimals))
    }

    /// x.exp() is e to the power of x
    ///
    /// Resources
    /// https://www.netlib.org/fdlibm/e_exp.c
    /// http://developer.classpath.org/doc/java/lang/StrictMath-source.html
    fn exp(x: Amount) -> Result<Amount, MathError> {
        let high: Amount;
        let low: Amount;
        let k: i64;
        let t = x.clone();
        let half = Amount::half(x.decimals)?;
        let ln2 = Amount::ln2(x.decimals);
        let one = Amount::one(x.decimals);
        let ln2_inv = Amount::ln2_inv(x.decimals);

        Ok(Amount::one(1))
    }

    /// Set the number of decimals, example mantissa 2000 decimals 3 (2 with 3 decimals of precision)
    /// call set_decimals(2) -> mantissa 200 decimals 2
    ///
    /// Note: This will result in truncation when the decimals are decreasing and zero padding
    /// when the decimals are increasing
    ///
    /// todo: do we need additional rounding modes?
    pub fn set_decimals<T: Into<DecimalType> + Copy>(self: &Self, new_decimals: T) -> Self {
        let new_decimals: DecimalType = new_decimals.into();

        if new_decimals == self.decimals {
            // not actually changing the decimals at all, just return the current value without doing
            // any work
            return self.clone();
        }
        let new_mantissa: MantissaType;
        if new_decimals > self.decimals {
            // increasing precision, multiply by a power of 10 to properly scale
            // note: difference_in_decimals does not need to be checked for underflow due to the
            // check above
            let difference_in_decimals = new_decimals - self.decimals;
            let pow_ten = 10u64.pow(difference_in_decimals as u32);
            new_mantissa = self.clone().mantissa * pow_ten;
        } else {
            // new_decimals < self.decimals
            // decreasing precision, divide by a power of 10 to properly scale
            // note: difference_in_decimals does not need to be checked for underflow due to the
            // logic of this function
            let difference_in_decimals = self.decimals - new_decimals;
            let pow_ten = 10u64.pow(difference_in_decimals as u32);
            new_mantissa = self.clone().mantissa / pow_ten;
        }

        Amount {
            mantissa: new_mantissa,
            decimals: new_decimals,
        }
    }

    /* --- Constants --- */

    // todo: make these compile time constants somehow
    /// A helper function to get a constant with a specific scaled value. Let's say you wanted
    /// to encode two as a constant with the natural decimals of zero, use the pseudocode
    /// to get the idea
    ///
    /// let two_func = |dec| { Amount::get_constant(dec, 0u8, 2u8) };
    ///
    /// Now you have a function that can give you the number two with any number of decimals
    /// Let's say ten decimals
    ///
    /// let two_with_ten_decimals = two_func(10);
    ///
    fn get_constant<
        D1: Into<DecimalType> + Copy,
        D2: Into<DecimalType> + Copy,
        M: Into<MantissaType>,
    >(
        decimals: D1,
        natural_decimals: D2,
        mantissa: M,
    ) -> Amount {
        let decimals: DecimalType = decimals.into();
        let natural_decimals: DecimalType = natural_decimals.into();

        let pre_scaled = Amount {
            mantissa: mantissa.into(),
            decimals: natural_decimals,
        };
        if decimals == natural_decimals {
            return pre_scaled;
        }
        let scaled = pre_scaled.set_decimals(decimals);

        scaled
    }

    /// the natural log of 2
    ///
    /// ln(2)
    fn ln2<D1: Into<DecimalType> + Copy>(decimals: D1) -> Amount {
        // 0.6931471805599453 16 dec
        Amount::get_constant(decimals, 16u8, 6931471805599453u64)
    }

    /// the inverse of the natural log of 2
    ///
    /// 1/ln(2)
    fn ln2_inv<D1: Into<DecimalType> + Copy>(decimals: D1) -> Amount {
        // 1.4426950408889634 16 dec
        Amount::get_constant(decimals, 16u8, 14426950408889634u64)
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
    fn test_half() -> Result<(), MathError> {
        let a = Amount::half(3)?;

        assert_eq!(a.decimals, 3);
        assert_eq!(a.mantissa, 500u32.into());

        Ok(())
    }

    #[test]
    fn test_mul() -> Result<(), MathError> {
        let a = Amount::new(30000_u32, 4);
        let b = Amount::new(20000_u32, 4);
        let expected = Amount::new(60000_u32, 4);
        let actual = a.mul(&b)?;
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_set_decimals_equal() {
        // 1.23456789 as an amount
        let a = Amount::new(123456789u64, 8);
        let b = a.set_decimals(8);
        assert_eq!(a, b);
    }

    #[test]
    fn test_set_decimals_increase_precision() {
        // 1.23456789 as an amount
        let a = Amount::new(123456789u64, 8);
        let actual = a.set_decimals(10);
        let expected = Amount::new(12345678900u64, 10);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_set_decimals_decrease_precision() {
        // 1.23456789 as an amount
        let a = Amount::new(123456789u64, 8);
        let actual = a.set_decimals(6);
        let expected = Amount::new(1234567u64, 6);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_ln2_with_decreased_precision() {
        let actual = Amount::ln2(3);
        let expected = Amount::new(693u32, 3);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_ln2_with_increased_precision() {
        let actual = Amount::ln2(18);
        let expected = Amount::new(693147180559945300u64, 18);
        assert_eq!(actual, expected);
    }
}
