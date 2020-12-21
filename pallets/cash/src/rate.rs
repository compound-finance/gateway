use crate::amount::{Amount, DecimalType, MathError};
use num_traits::{Pow, ToPrimitive};

/// 1 in decimal is 10000 basis points
pub const DECIMAL_TO_BASIS_POINTS: f64 = 10000f64;
/// Approximate number of seconds in one year.
/// 1 year = 365.2425 days = (365.2425 days) × (24 hours/day) × (3600 seconds/hour) = 31556952 seconds
pub const SECONDS_PER_YEAR: u64 = 31556952u64;

/// Struct to store the interest rate on the compound chain. This interest rate may go negative. This
/// interest rate is a number that represents something like 3% APR and is changed via governance.
/// In that sense it does not change naturally as time progresses as the interest index does.
///
/// The rate is stored as an i16 and is stored as BASIS POINTS as an INTEGER. The extremum are
/// plus or minus 163.84% per year.
struct Rate {
    basis_points_per_year: i16,
}

impl Rate {
    /// Create a new interest rate
    pub fn new(basis_points_per_year: i16) -> Rate {
        Rate {
            basis_points_per_year,
        }
    }

    /// Convert a rate to a float as decimals. For example if your rate is 300 basis points
    /// then this function returns 0.03f64. This is useful for interest rate calculations.
    pub fn to_f64_as_decimal(self: &Self) -> Result<f64, MathError> {
        let mantissa: f64 = self
            .basis_points_per_year
            .to_f64()
            .ok_or(MathError::ConversionError)?;
        let result = mantissa / DECIMAL_TO_BASIS_POINTS;
        Ok(result)
    }

    /// The exponential function applied to the current value. If the current value is 300 basis points
    /// then this function returns exp(0.03)
    pub fn exp(self: &Self, destination_decimals: DecimalType) -> Result<Amount, MathError> {
        self.to_index(destination_decimals, SECONDS_PER_YEAR)
    }

    /// Your rate is something like 3% annualized. This function converts to an index value by using
    /// a "delta T" that the rate applies across. So you say, ok, 3% per year applied for 10,000
    /// seconds yields an index multiplier of exp(0.03 * 10000 /31556952) = 1.00000950666674. This
    /// value is returned to the caller as an Amount with the requested number of decimals.
    pub fn to_index(
        self: &Self,
        destination_decimals: DecimalType,
        time_in_seconds: u64,
    ) -> Result<Amount, MathError> {
        // this is something like 0.03 now
        let converted = self.to_f64_as_decimal()?;
        // Now, need to scale by our time factor, because our interest rate is represented as DECIMAL
        // we do not need to "divide by 100" as you would with percent. Because our interest rate
        // is annualized, we must annualize our time as well, the given time is in units of seconds
        // so it must be converted to YEARS by dividing by SECONDS_PER_YEAR.
        let scaled = converted * (time_in_seconds as f64) / (SECONDS_PER_YEAR as f64);
        let result = scaled.exp();
        let result_converted = Amount::from_f64_lossy(result, destination_decimals)?;

        Ok(result_converted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate_from_percent(percent: i16) -> Rate {
        Rate::new(percent * 100)
    }

    #[test]
    fn test_into_float() {
        let one_half = rate_from_percent(50);
        let expected = 0.5f64;
        let actual: f64 = one_half.to_f64_as_decimal().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_to_index() {
        let three_percent = rate_from_percent(3);
        let six_months_in_seconds = SECONDS_PER_YEAR / 2;
        let expected = (0.03f64 / 2f64).exp();
        let expected = Amount::from_f64_lossy(expected, 18).unwrap();

        let actual = three_percent.to_index(18, six_months_in_seconds).unwrap();
        assert_eq!(expected, actual);
    }
}
