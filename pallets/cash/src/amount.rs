use anyhow::{bail, Result};
use codec::{Decode, Encode, Input};
use num_bigint::BigUint;
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use sp_std::vec::Vec;

/// The type of the decimal field.
pub type DecimalType = u8;

/// The type of the mantissa field
pub type MantissaType = BigUint;

/// The type for Cash
pub type CashAmount = u128;

const CASH_DECIMALS: DecimalType = 18;

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
    InsufficientPrecision,
    Underflow,
    Overflow,
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
    pub fn new_cash<T: Into<MantissaType>>(mantissa: T) -> Self {
        Amount {
            decimals: CASH_DECIMALS,
            mantissa: mantissa.into(),
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

    /// Subtract two FixedPrecision numbers together. Note the signature uses borrowed values this is
    /// because the underlying storage is arbitrarily large and we do not want to copy the values.
    pub fn sub(self: &Self, rhs: &Self) -> Result<Self, MathError> {
        self.check_decimals(rhs)?;
        if self.lt(rhs)? {
            return Err(MathError::Underflow);
        }
        // note - this cannot fail with BigUint but that will change based on underlying storage
        let new_mantissa = &self.mantissa - &rhs.mantissa;

        Ok(Self::new(new_mantissa, self.decimals))
    }

    /// Multiply two numbers with the same number of decimals of precision
    /// Example 2 times 2 with 4 digits of precision -> mantissa 40000 digits 4
    pub fn mul(self: &Self, rhs: &Self) -> Result<Self, MathError> {
        self.check_decimals(rhs)?;

        let one = Self::one(self.decimals);

        // need to divide by the mantissa of one to properly scale
        // todo: is there a better way to do this without having to compute so many useless digits?
        let new_mantissa = &self.mantissa * &rhs.mantissa / one.mantissa;

        Ok(Self::new(new_mantissa, self.decimals))
    }

    /// Divide two numbers with the same number of decimals of precision
    ///
    /// The rounding mode is always truncation
    pub fn div(self: &Self, rhs: &Self) -> Result<Self, MathError> {
        self.check_decimals(rhs)?;

        // this is safe because we just checked that decimals is not zero
        let one = Self::one(self.decimals);

        // need to multiply by the mantissa of one before division to properly scale
        let new_mantissa = &self.mantissa * &one.mantissa / &rhs.mantissa;

        Ok(Self::new(new_mantissa, self.decimals))
    }

    /// greater than function
    ///
    /// a.gt(b) is the same as a > b
    pub fn gt(self: &Self, rhs: &Self) -> Result<bool, MathError> {
        self.check_decimals(rhs)?;

        Ok(self.mantissa > rhs.mantissa)
    }

    /// less than function
    ///
    /// a.gt(b) is the same as a < b
    pub fn lt(self: &Self, rhs: &Self) -> Result<bool, MathError> {
        self.check_decimals(rhs)?;

        Ok(self.mantissa < rhs.mantissa)
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
    pub fn one_half<T: Into<DecimalType> + Copy>(decimals: T) -> Result<Self, MathError> {
        let decimals: DecimalType = decimals.into();
        if decimals == 0 {
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
        let one = Amount::one(x.decimals);
        let zero = Amount::zero(x.decimals);
        if x == zero {
            return Ok(one);
        }

        let high: Amount;
        let low: Amount;
        let k: Amount;
        let t = x.clone();
        let half = Amount::one_half(x.decimals)?;
        let ln2 = Amount::ln2(x.decimals);
        let ln2_high = Amount::ln2_high(x.decimals);
        let ln2_low = Amount::ln2_low(x.decimals);
        let ln2_inv = Amount::ln2_inv(x.decimals);
        let one_point_five = one.add(&half)?;

        if t.gt(&half.mul(&ln2)?)? {
            if t.lt(&one_point_five.mul(&ln2)?)? {
                k = one.clone();
                high = t.sub(&ln2_high)?;
                low = ln2_low;
            } else {
                k = ln2_inv.mul(&t)?.add(&half)?;
                high = t.sub(&k.mul(&ln2_high)?)?;
                low = k.mul(&ln2_low)?;
            }
        } else {
            // low = high = k = 0
            low = zero.clone();
            high = zero.clone();
            k = zero.clone();
        }

        // compute the polynomial
        // note terms p2 and p4 are actually negative so they must be subtracted
        // this is a departure from the implementation for floats since they can represent negative numbers
        // the hope is that the intermediate representation within this approximation
        // does not require negative values and I don't believe that it will

        let p1 = Amount::p1(x.decimals);
        let p2_neg = Amount::p2_neg(x.decimals);
        let p3 = Amount::p3(x.decimals);
        let p4_neg = Amount::p4_neg(x.decimals);
        let p5 = Amount::p5(x.decimals);

        // the accumulator will help us compute the polynomial in a readable way
        // the number in these variable names represents the power eg t^2 t^4 etc
        let t2 = x.mul(&x)?;
        let t4 = t2.mul(&t2)?;
        let t6 = t4.mul(&t2)?;
        let t8 = t6.mul(&t2)?;
        let t10 = t8.mul(&t2)?;

        // set up the positive terms first
        let pos_sum = t2.mul(&p1)?.add(&t6.mul(&p3)?)?.add(&t10.mul(&p5)?)?;
        // set up negative terms
        let neg_sum = t4.mul(&p2_neg)?.add(&t8.mul(&p4_neg)?)?;
        // let's hope the positive terms are greater than the negative terms (they should be..?)
        let total = pos_sum.sub(&neg_sum)?;

        let c = x.sub(&total)?;

        let two = one.add(&one)?;
        // x * c / (c - 2)
        let frac = x.mul(&c.div(&c.sub(&two)?)?)?;
        if k == zero {
            // 1 - (x * c / (c - 2) - x)
            let ans = one.sub(&frac.sub(&x)?);
            return ans;
        }
        // y = 1 - (lo - x * c / (2 - c) - hi)
        let y = one.sub(&low.sub(&frac)?.sub(&high)?)?;
        // need to floor k
        let k_floored = k
            .set_decimals(0u8)
            .mantissa
            .to_u32()
            .ok_or(MathError::Overflow)?;

        // need y * 2^k
        // todo: should this be done with bitshifting ... probably but i'm a noob
        let (scalar, overflowed) = 2u64.overflowing_pow(k_floored);
        if overflowed {
            return Err(MathError::Overflow);
        }
        let scalar = Amount::new(scalar, 0);
        let scalar = scalar.set_decimals(x.decimals);
        let ans = y.mul(&scalar)?;

        return Ok(ans);
    }

    /// Set the number of decimals, example mantissa 2000 decimals 3 (2 with 3 decimals of precision)
    /// call set_decimals(2) -> mantissa 200 decimals 2
    ///
    /// Note: This will result in truncation when the decimals are decreasing and zero padding
    /// when the decimals are increasing
    ///
    /// todo: do we need additional rounding modes?
    pub fn set_decimals<T: Into<DecimalType> + Copy>(self: Self, new_decimals: T) -> Self {
        let new_decimals: DecimalType = new_decimals.into();

        if new_decimals == self.decimals {
            // not actually changing the decimals at all, just return the current value without doing
            // any work
            return self;
        }
        let new_mantissa: MantissaType;
        if new_decimals > self.decimals {
            // increasing precision, multiply by a power of 10 to properly scale
            // note: difference_in_decimals does not need to be checked for underflow due to the
            // check above
            let difference_in_decimals = new_decimals - self.decimals;
            let pow_ten = 10u64.pow(difference_in_decimals as u32);
            new_mantissa = self.mantissa * pow_ten;
        } else {
            // new_decimals < self.decimals
            // decreasing precision, divide by a power of 10 to properly scale
            // note: difference_in_decimals does not need to be checked for underflow due to the
            // logic of this function
            let difference_in_decimals = self.decimals - new_decimals;
            let pow_ten = 10u64.pow(difference_in_decimals as u32);
            new_mantissa = self.mantissa / pow_ten;
        }

        Amount {
            mantissa: new_mantissa,
            decimals: new_decimals,
        }
    }

    /* --- Constants --- */

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

    /// the zero value
    fn zero<D1: Into<DecimalType> + Copy>(decimals: D1) -> Amount {
        // 0.6931471805599453 16 dec
        Amount {
            mantissa: 0u64.into(),
            decimals: decimals.into(),
        }
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

    /// a bound for the estimating polynomial within the exp function
    fn ln2_high<D1: Into<DecimalType> + Copy>(decimals: D1) -> Amount {
        // 0.6931471803691238 16 dec
        Amount::get_constant(decimals, 16u8, 6931471803691238u64)
    }

    /// a bound for the estimating polynomial within the exp function
    fn ln2_low<D1: Into<DecimalType> + Copy>(decimals: D1) -> Amount {
        // 0.00000000019082149292705877 26 dec
        Amount::get_constant(decimals, 26u8, 19082149292705877u64)
    }

    fn p1<D1: Into<DecimalType> + Copy>(decimals: D1) -> Amount {
        // 0.6931471803691238 16 dec
        Amount::get_constant(decimals, 17u8, 16666666666666602u64)
    }
    fn p2_neg<D1: Into<DecimalType> + Copy>(decimals: D1) -> Amount {
        // 0.6931471803691238 16 dec
        Amount::get_constant(decimals, 19u8, 27777777777015593u64)
    }
    fn p3<D1: Into<DecimalType> + Copy>(decimals: D1) -> Amount {
        // 0.6931471803691238 16 dec
        Amount::get_constant(decimals, 20u8, 6613756321437934u64)
    }
    fn p4_neg<D1: Into<DecimalType> + Copy>(decimals: D1) -> Amount {
        // 0.6931471803691238 16 dec
        Amount::get_constant(decimals, 22u8, 16533902205465252u64)
    }
    fn p5<D1: Into<DecimalType> + Copy>(decimals: D1) -> Amount {
        // 0.6931471803691238 16 dec
        Amount::get_constant(decimals, 24u8, 41381367970572385u64)
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
    fn test_one_half() -> Result<(), MathError> {
        let a = Amount::one_half(3)?;

        assert_eq!(a.decimals, 3);
        assert_eq!(a.mantissa, 500u32.into());

        Ok(())
    }

    #[test]
    fn test_one_half_insufficient_precision() {
        let actual = Amount::one_half(0);
        let expected = Err(MathError::InsufficientPrecision);
        assert_eq!(expected, actual);
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
        let b = a.clone().set_decimals(8);
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

    #[test]
    fn test_scale_codec() -> Result<(), codec::Error> {
        let expected = Amount::new(6000u32, 3);
        let encoded = expected.encode();
        let actual: Amount = Decode::decode(&mut encoded.as_slice())?;
        assert_eq!(expected, actual);

        Ok(())
    }

    fn inequality_test_helper(
        a: &Amount,
        b: &Amount,
        expected_lt: bool,
        expected_gt: bool,
    ) -> Result<(), MathError> {
        let actual_lt = a.lt(b)?;
        let actual_gt = a.gt(b)?;
        assert_eq!(expected_lt, actual_lt);
        assert_eq!(expected_gt, actual_gt);

        Ok(())
    }

    #[test]
    fn test_gt_true_when_greater() -> Result<(), MathError> {
        let a = Amount::new(30000_u32, 4);
        let b = Amount::new(20000_u32, 4);
        inequality_test_helper(&a, &b, false, true)?;

        Ok(())
    }

    #[test]
    fn test_gt_false_when_equal() -> Result<(), MathError> {
        let a = Amount::new(30000_u32, 4);
        let b = Amount::new(30000_u32, 4);
        inequality_test_helper(&a, &b, false, false)?;

        Ok(())
    }

    #[test]
    fn test_gt_false_when_less() -> Result<(), MathError> {
        let a = Amount::new(30000_u32, 4);
        let b = Amount::new(40000_u32, 4);
        inequality_test_helper(&a, &b, true, false)?;

        Ok(())
    }

    #[test]
    fn test_sub_happy_path() -> Result<(), MathError> {
        let a = Amount::new(30000_u32, 4);
        let b = Amount::new(40000_u32, 4);
        let expected = Amount::new(10000_u32, 4);
        let actual = b.sub(&a)?;
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_sub_numbers_equal() -> Result<(), MathError> {
        let a = Amount::new(40000_u32, 4);
        let b = Amount::new(40000_u32, 4);
        let expected = Amount::new(0_u32, 4);
        let actual = b.sub(&a)?;
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_sub_underflow() {
        let a = Amount::new(30000_u32, 4);
        let b = Amount::new(40000_u32, 4);
        let expected = Err(MathError::Underflow);
        let actual = a.sub(&b);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_div_happy_path() -> Result<(), MathError> {
        let a = Amount::new(30000_u32, 4);
        let b = Amount::new(40000_u32, 4);
        let expected = Amount::new(7500u32, 4);
        let actual = a.div(&b)?;
        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn test_exp() -> Result<(), MathError> {
        let x = Amount::one(18);
        let e = Amount::exp(x)?;

        Ok(())
    }
}
