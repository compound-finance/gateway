use crate::amount::{Amount, DecimalType, MathError};
use num_traits::{Pow, ToPrimitive};

/// The interest rate number of decimals as a float eg 12 decimals of precision
pub const RATE_DECIMALS_FLOAT: f64 = 12f64;
/// The interest rate mantissa scalar for rates as stored in i64 eg 1,000,000,000,000 1 trillion
pub const RATE_ONE_MANTISSA: u64 = 1000000000000;
/// Approximate number of seconds in one year.
/// 1 year = 365.2425 days = (365.2425 days) × (24 hours/day) × (3600 seconds/hour) = 31556952 seconds
pub const SECONDS_PER_YEAR: u64 = 31556952u64;

/// Struct to store the interest rate on the compound chain. This interest rate may go negative. This
/// interest rate is a number that represents something like 3% APY and is changed via governance.
/// In that sense it does not change as time progresses as the interest index does.
///
/// The rate is stored as an i64 and is stored as DECIMAL type as opposed to PERCENT or BASIS POINTS.
/// So if the RATE_DECIMALS_FLOAT value is 5 and we wanted to store 3 percent per year as our rate
/// the value of the Mantissa field should be 3000. The rate is signed and as such it may become
/// negative. Calling code should expect this case and account for it.
struct Rate {
    mantissa: i64,
}

impl Rate {
    pub fn new(mantissa: i64) -> Rate {
        Rate { mantissa }
    }

    pub fn to_f64(self: &Self) -> Result<f64, MathError> {
        let mantissa: f64 = self.mantissa.to_f64().ok_or(MathError::ConversionError)?;
        let ten = 10f64;
        let pow_ten: f64 = ten.pow(RATE_DECIMALS_FLOAT);
        let result = mantissa / pow_ten;
        Ok(result)
    }

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
        let converted = self.to_f64()?;
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

    fn rate_from_percent(percent: i64) -> Rate {
        Rate::new(percent * (RATE_ONE_MANTISSA as i64) / 100i64)
    }

    #[test]
    fn test_into_float() {
        let one_half = rate_from_percent(50);
        let expected = 0.5f64;
        let actual: f64 = one_half.to_f64().unwrap();
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
