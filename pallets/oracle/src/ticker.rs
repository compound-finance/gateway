use codec::{Decode, Encode};
use our_std::{
    convert::TryInto,
    fixed_width::{label_to_string, str_to_label, WIDTH},
    RuntimeDebug,
};

use types_derive::Types;

use crate::error::OracleError;

/// Type for an asset price ticker.
#[derive(Copy, Clone, Eq, Encode, Decode, PartialEq, Ord, PartialOrd, RuntimeDebug, Types)]
pub struct Ticker(pub [u8; 12]);

impl Ticker {
    pub const fn new(ticker_str: &str) -> Self {
        Ticker(str_to_label(ticker_str))
    }
}

impl our_std::str::FromStr for Ticker {
    type Err = OracleError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars: Vec<u8> = s.as_bytes().into();
        if chars.len() > WIDTH {
            Err(OracleError::BadTicker)
        } else {
            chars.resize(WIDTH, 0);
            Ok(Ticker(chars.try_into().unwrap()))
        }
    }
}

impl From<Ticker> for String {
    fn from(ticker: Ticker) -> String {
        label_to_string(ticker.0)
    }
}

pub const USD_TICKER: Ticker = Ticker::new("USD");
pub const CASH_TICKER: Ticker = Ticker::new("CASH");
