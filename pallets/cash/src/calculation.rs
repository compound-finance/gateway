use crate::reason::MathError;
use crate::types::Decimals;
use num_bigint::BigInt;
use num_traits::ToPrimitive;

pub struct Calculation {
    numerator: BigInt,
    denominator: BigInt,
}

impl Default for Calculation {
    fn default() -> Self {
        Calculation {
            numerator: BigInt::from(1),
            denominator: BigInt::from(1),
        }
    }
}

impl Calculation {
    pub fn new() -> Calculation {
        Default::default()
    }

    pub fn mul(mut self, raw: i128, decimals: Decimals) -> Self {
        self.numerator = self.numerator * raw;
        self.denominator = self.denominator * 10u128.pow(decimals as u32);

        self
    }

    pub fn div(mut self, raw: i128, decimals: Decimals) -> Self {
        self.denominator = self.denominator * raw;
        self.numerator = self.numerator * 10u128.pow(decimals as u32);

        self
    }

    pub fn calc_u128(self, output_scale: Decimals) -> Result<u128, MathError> {
        let raw = self.numerator * 10u128.pow(output_scale as u32) / self.denominator;
        raw.to_u128().ok_or(MathError::Overflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cash_principal_per_calc() {
        let calc = Calculation::new()
            .mul(1234, 4) // cost 12.34 % apr
            .mul(6000, 0) // // 6 seconds as ms
            .mul(1234_123456, 6) // $1234.123456 (eth lets say)
            .div(1_012345678912345678, 18) // 1.01234... cash index
            .div(1_000000) // $1 cash
            .div(365 * 24 * 60 * 60 * 1000, 0) // ms per year
            .calc_u128(18)
            .expect("should not overflow");

        // sheet had 28621314533265 last dec off by 1
        // I think the sheet rounded
        assert_eq!(calc, 28621314533264);
    }
}
