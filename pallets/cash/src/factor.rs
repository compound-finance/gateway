use codec::{Decode, Encode};
use num_bigint::{BigInt as BigI, BigUint as BigU};
use num_traits::{CheckedDiv, ToPrimitive};
use our_std::{consts::uint_from_string_with_decimals, Deserialize, RuntimeDebug, Serialize};

use crate::{
    reason::{MathError, Reason},
    types::{Decimals, Int, Uint},
};

use types_derive::Types;

/// Type for wrapping intermediate signed calculations.
pub struct BigInt(pub BigI);

impl BigInt {
    pub fn from_uint(u: Uint) -> Self {
        BigInt(BigI::from(u))
    }

    pub fn mul_decimal(self, number: Int, decimals: Decimals) -> Self {
        let scale = 10i128.pow(decimals as u32);
        BigInt((self.0) * number / scale)
    }

    pub fn div_decimal(self, number: Int, decimals: Decimals) -> Result<Self, MathError> {
        let scale = 10i128.pow(decimals as u32);
        Ok(BigInt(
            (self.0 * scale)
                .checked_div(&BigI::from(number))
                .ok_or(MathError::DivisionByZero)?,
        ))
    }

    pub fn convert(self, from_decimals: Decimals, to_decimals: Decimals) -> Self {
        if from_decimals > to_decimals {
            BigInt(self.0 / 10i128.pow((from_decimals - to_decimals) as u32))
        } else {
            BigInt(self.0 * 10i128.pow((to_decimals - from_decimals) as u32))
        }
    }

    pub fn to_int(self) -> Result<Int, MathError> {
        (self.0).to_i128().ok_or(MathError::Overflow)
    }
}

/// Type for wrapping intermediate calculations.
pub struct BigUint(pub BigU);

impl BigUint {
    pub fn from_uint(u: Uint) -> Self {
        BigUint(BigU::from(u))
    }

    pub fn mul_decimal(self, number: Uint, decimals: Decimals) -> Self {
        let scale = 10u128.pow(decimals as u32);
        BigUint((self.0) * number / scale)
    }

    pub fn div_decimal(self, number: Uint, decimals: Decimals) -> Result<Self, MathError> {
        let scale = 10u128.pow(decimals as u32);
        Ok(BigUint(
            (self.0 * scale)
                .checked_div(&BigU::from(number))
                .ok_or(MathError::DivisionByZero)?,
        ))
    }

    pub fn mul_uint(self, number: Uint) -> Self {
        BigUint((self.0) * number)
    }

    pub fn div_uint(self, number: Uint) -> Result<Self, MathError> {
        Ok(BigUint(
            (self.0)
                .checked_div(&BigU::from(number))
                .ok_or(MathError::DivisionByZero)?,
        ))
    }

    pub fn convert(self, from_decimals: Decimals, to_decimals: Decimals) -> Self {
        if from_decimals > to_decimals {
            BigUint(self.0 / 10u128.pow((from_decimals - to_decimals) as u32))
        } else {
            BigUint(self.0 * 10u128.pow((to_decimals - from_decimals) as u32))
        }
    }

    pub fn to_uint(self) -> Result<Uint, MathError> {
        (self.0).to_u128().ok_or(MathError::Overflow)
    }
}

/// Type for an unsigned factor, with a large fixed number of decimals.
#[derive(Serialize, Deserialize)] // used in config
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug, Types)]
pub struct Factor(pub Uint);

impl Factor {
    pub const DECIMALS: Decimals = 18;
    pub const ZERO: Factor = Factor::from_nominal("0");
    pub const ONE: Factor = Factor::from_nominal("1");

    /// Get a factor from a string.
    /// Only for use in const contexts.
    pub const fn from_nominal(s: &'static str) -> Self {
        Factor(uint_from_string_with_decimals(Self::DECIMALS, s))
    }

    pub fn from_fraction<T: Into<Uint>>(numerator: T, denominator: T) -> Result<Self, MathError> {
        Ok(Factor(
            Self::ONE
                .mul_uint(numerator.into())
                .div_uint(denominator.into())?
                .to_uint()?,
        ))
    }

    pub fn mul(self, rhs: Factor) -> BigUint {
        self.mul_decimal(rhs.0, Self::DECIMALS)
    }

    pub fn mul_decimal(self, number: Uint, decimals: Decimals) -> BigUint {
        BigUint::from_uint(self.0).mul_decimal(number, decimals)
    }

    pub fn mul_uint(self, number: Uint) -> BigUint {
        BigUint::from_uint(self.0).mul_uint(number)
    }
}

impl Default for Factor {
    fn default() -> Self {
        Factor::ZERO
    }
}

impl our_std::str::FromStr for Factor {
    type Err = Reason;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Ok(Factor(
            u128::from_str(string).map_err(|_| Reason::BadFactor)?,
        ))
    }
}

impl From<Factor> for String {
    fn from(string: Factor) -> Self {
        format!("{}", string.0)
    }
}

impl From<Uint> for Factor {
    fn from(raw: u128) -> Self {
        Factor(raw)
    }
}
