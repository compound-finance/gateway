use codec::{Decode, Encode};
use num_bigint::BigUint;
use our_std::{
    convert::TryFrom,
    ops::{Add, Div, Mul, Sub},
    RuntimeDebug,
};

// TODO: This conflicts with `Uint` defined in types.rs
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug)]
pub struct Uint(pub BigUint);

impl Encode for Uint {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.0.to_bytes_le().using_encoded(f)
    }
}

impl codec::EncodeLike for Uint {}

impl Decode for Uint {
    fn decode<I: codec::Input>(encoded: &mut I) -> Result<Self, codec::Error> {
        let raw: Vec<u8> = Decode::decode(encoded)?;
        Ok(Uint(BigUint::from_bytes_le(&raw)))
    }
}

impl<T> From<T> for Uint
where
    T: Into<BigUint>,
{
    fn from(raw: T) -> Self {
        Uint(raw.into())
    }
}

impl TryFrom<Uint> for u128 {
    type Error = num_bigint::TryFromBigIntError<BigUint>;

    fn try_from(from: Uint) -> Result<u128, Self::Error> {
        TryFrom::try_from(from.0)
    }
}

impl<T> Add<T> for Uint
where
    T: Into<Uint>,
{
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Uint(self.0 + rhs.into().0)
    }
}

impl<T> Sub<T> for Uint
where
    T: Into<Uint>,
{
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        Uint(self.0 - rhs.into().0)
    }
}

impl<T> Mul<T> for Uint
where
    T: Into<Uint>,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Uint(self.0 * rhs.into().0)
    }
}

impl<T> Div<T> for Uint
where
    T: Into<Uint>,
{
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Uint(self.0 / rhs.into().0)
    }
}
