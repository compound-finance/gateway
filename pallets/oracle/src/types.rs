use crate::{error::OracleError, ticker::Ticker};
use codec::{Decode, Encode};
use our_std::{consts::uint_from_string_with_decimals, convert::TryFrom, RuntimeDebug};

use types_derive::{type_alias, Types};

/// Type for an open price feed reporter.

#[type_alias]
pub type Reporter = [u8; 20];

/// Type for representing time since current Unix epoch in milliseconds.
#[type_alias("Oracle__")]
pub type Timestamp = u64;

/// Type for representing a price, potentially for any symbol.
#[type_alias]
pub type AssetPrice = u128;

// XXX ideally we should really impl Ord ourselves for these
//  and should assert ticker/units is same when comparing
//   would have to panic, though not for partial ord

/// Type for representing a price (in USD), bound to its ticker.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug, Types)]
pub struct Price {
    pub ticker: Ticker,
    pub value: AssetPrice,
}

impl Price {
    pub const DECIMALS: u8 = 6;

    pub const fn new(ticker: Ticker, value: AssetPrice) -> Self {
        Price { ticker, value }
    }

    /// Get a price from a string.
    /// Only for use in const contexts.
    pub const fn from_nominal(ticker: Ticker, s: &'static str) -> Self {
        Price::new(ticker, uint_from_string_with_decimals(Self::DECIMALS, s))
    }
}

/// Type for a set of open price feed reporters.
#[derive(Clone, Eq, PartialEq, Encode, Decode, Default, RuntimeDebug, Types)]
pub struct ReporterSet(pub Vec<Reporter>);

impl ReporterSet {
    pub fn contains(&self, reporter: Reporter) -> bool {
        self.0.iter().any(|e| e.as_slice() == reporter.as_slice())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> TryFrom<Vec<&'a str>> for ReporterSet {
    type Error = OracleError;
    fn try_from(strings: Vec<&'a str>) -> Result<ReporterSet, Self::Error> {
        let mut reporters = Vec::with_capacity(strings.len());
        for string in strings {
            reporters
                .push(gateway_crypto::str_to_address(string).ok_or(OracleError::InvalidReporter)?)
        }
        Ok(ReporterSet(reporters))
    }
}
