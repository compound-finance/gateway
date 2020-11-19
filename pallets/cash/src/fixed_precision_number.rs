use primitive_types::U256;

/// The math error type
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MathError {
    Overflow,
    PrecisionMismatch,
}

/// The SafeMath trait allows us to have a consistent way to do safe math operations between
/// different storage types including primitives u8 u32 u64 as well as parity primitives
/// U128 U256 U512. This unified interface allows us to implement other safe algorithms on
/// underlying types such as exp and integer powers in a fully generic way (static dispatch)
///
/// There will most likely be some performance improvements we want to make here such as
/// #[inline_always] annotations but much of that should come naturally from compiler optimization
pub trait SafeMath {
    fn add(self, rhs: Self) -> Result<Self, MathError>
    where
        Self: Sized; // required due to Result
}

/// Implement SafeMath for existing primitive types (will likely need to go into a macro :sad:)
impl SafeMath for U256 {
    fn add(self, rhs: Self) -> Result<Self, MathError> {
        convert_overflowing(self.overflowing_add(rhs))
    }
}

impl SafeMath for u8 {
    fn add(self, rhs: Self) -> Result<Self, MathError> {
        self.checked_add(rhs).ok_or(MathError::Overflow)
    }
}

/// The FixedPrecisionNumber represents numbers using a base type to store the mantissa
/// with a u8 for the number of decimals of precision. For example, if we use
/// FixedPrecisionNumber<u8> we can represent
#[derive(Copy, Clone, Debug)]
pub struct FixedPrecisionNumber<T: SafeMath> {
    mantissa: T,
    decimals: u8,
}

/// Constructor for FixedPrecisionNumber
pub fn new_fixed_precision_number<T: SafeMath>(
    mantissa: T,
    decimals: u8,
) -> FixedPrecisionNumber<T> {
    FixedPrecisionNumber { mantissa, decimals }
}

/// check that the decimals on two FixedPrecisionNumbers are the same, error if not.
fn check_decimals<T: SafeMath>(
    lhs: FixedPrecisionNumber<T>,
    rhs: FixedPrecisionNumber<T>,
) -> Result<(), MathError> {
    if lhs.decimals != rhs.decimals {
        Err(MathError::PrecisionMismatch)
    } else {
        Ok(())
    }
}

/// The primitive_types crate from substrate has several overflowing_* functions that return
/// tuples. This is antithetical to the general practice of error handling in rust generally
/// speaking as well as even being inconsistent with the general convention of the
/// standard libraries checked_* functions on primitives. This function converts the return
/// values from tuples to results for easy use with the ? operator downstream.
fn convert_overflowing<T>(overflowing_result: (T, bool)) -> Result<T, MathError> {
    let (value, is_overflow) = overflowing_result;
    if is_overflow {
        Err(MathError::Overflow)
    } else {
        Ok(value)
    }
}

impl SafeMath for FixedPrecisionNumber<U256> {
    fn add(self, rhs: Self) -> Result<FixedPrecisionNumber<U256>, MathError> {
        check_decimals(self, rhs)?;
        let new_mantissa = convert_overflowing(self.mantissa.overflowing_add(rhs.mantissa))?;
        Ok(new_fixed_precision_number(new_mantissa, self.decimals))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_happy_path() -> Result<(), MathError> {
        // let's do an 18 decimal number on a U256
        let x = new_fixed_precision_number(U256::one(), 2);
        // error out if we overflow
        let twox = x.add(x)?;
        // make sure we have the correct expected value in the mantissa
        assert_eq!(twox.mantissa, U256::from(2));
        assert_eq!(twox.decimals, 2);

        Ok(())
    }

    #[test]
    fn test_overflow_u8() {
        let result = 255.add(1);
        assert!(result.is_err());
        let error = result.err().unwrap();
        assert_eq!(error, MathError::Overflow)
    }
}
