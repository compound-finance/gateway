use codec::{Decode, Encode};
use our_std::{convert::TryInto, RuntimeDebug};

use crate::{
    reason::Reason,
    types::{Decimals, Uint},
};

/// Fixed symbol width.
pub const WIDTH: usize = 12;

/// Type for the abstract symbol of an asset, not tied to a chain.
#[derive(Copy, Clone, Eq, Encode, Decode, PartialEq, Ord, PartialOrd, RuntimeDebug)]
pub struct Symbol(pub [u8; WIDTH]);

impl Symbol {
    pub const fn new(symbol_str: &str) -> Self {
        Symbol(str_to_label(symbol_str))
    }
}

impl our_std::str::FromStr for Symbol {
    type Err = Reason;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars: Vec<u8> = s.as_bytes().into();
        if chars.len() > WIDTH {
            Err(Reason::BadSymbol)
        } else {
            chars.resize(WIDTH, 0);
            Ok(Symbol(chars.try_into().unwrap()))
        }
    }
}

impl From<Symbol> for String {
    fn from(symbol: Symbol) -> String {
        label_to_string(symbol.0)
    }
}

/// Type for an asset price ticker.
#[derive(Copy, Clone, Eq, Encode, Decode, PartialEq, Ord, PartialOrd, RuntimeDebug)]
pub struct Ticker(pub [u8; WIDTH]);

impl Ticker {
    pub const fn new(ticker_str: &str) -> Self {
        Ticker(str_to_label(ticker_str))
    }
}

impl our_std::str::FromStr for Ticker {
    type Err = Reason;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars: Vec<u8> = s.as_bytes().into();
        if chars.len() > WIDTH {
            Err(Reason::BadTicker)
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

/// Type for determining whether quantities may be combined.
#[derive(Copy, Clone, Eq, Encode, Decode, PartialEq, Ord, PartialOrd, RuntimeDebug)]
pub struct Units {
    pub ticker: Ticker,
    pub decimals: Decimals,
}

// Define units used directly by the chain itself

/// Units for CASH.
pub const CASH: Units = Units::from_ticker_str("CASH", 18);

/// Units for USD.
pub const USD: Units = Units::from_ticker_str("USD", 6);

/// Statically get the Uint corresponding to some number of decimals.
pub const fn static_pow10(decimals: Decimals) -> Uint {
    let mut v: Uint = 1;
    let mut i = 0;
    loop {
        if i >= decimals {
            return v;
        }
        i += 1;
        v *= 10;
    }
}

pub const fn str_to_label(s: &str) -> [u8; WIDTH] {
    assert!(s.len() < WIDTH, "Too many chars");
    let mut bytes = [0u8; WIDTH];
    let mut i = 0;
    let raw = s.as_bytes();
    loop {
        if i >= s.len() {
            break;
        }
        bytes[i] = raw[i];
        i += 1;
    }
    bytes
}

pub fn label_to_string(l: [u8; WIDTH]) -> String {
    let mut s = String::with_capacity(WIDTH);
    let bytes = &l[..];
    for i in 0..WIDTH {
        if bytes[i] == 0 {
            break;
        }
        s.push(bytes[i] as char);
    }
    s
}

impl Units {
    pub const fn one(&self) -> Uint {
        static_pow10(self.decimals)
    }

    pub const fn new(ticker: Ticker, decimals: Decimals) -> Self {
        Units { ticker, decimals }
    }

    pub const fn from_ticker_str(ticker_str: &str, decimals: Decimals) -> Self {
        Units::new(Ticker::new(ticker_str), decimals)
    }
}

// Implement deserialization for Unitss so we can use them in GenesisConfig / ChainSpec JSON.
//  i.e. "TICKER/6" <> Units(["T", "I", "C", "K", "E", "R", 0, 0, 0, 0, 0, 0], 6)
impl our_std::str::FromStr for Units {
    type Err = Reason;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((ticker_str, decimal_str)) = String::from(s).split_once("/") {
            let mut chars: Vec<u8> = ticker_str.as_bytes().into();
            if chars.len() > WIDTH {
                Err(Reason::BadUnits)
            } else if let Ok(decimals) = decimal_str.parse::<u8>() {
                chars.resize(WIDTH, 0);
                Ok(Units::new(Ticker(chars.try_into().unwrap()), decimals))
            } else {
                Err(Reason::BadUnits)
            }
        } else {
            Err(Reason::BadUnits)
        }
    }
}

// For serialize (which we don't use, but are required to implement)
impl From<Units> for String {
    fn from(units: Units) -> String {
        format!("{}/{}", Into::<String>::into(units.ticker), units.decimals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_pow10() {
        assert_eq!(static_pow10(0), 1);
        assert_eq!(static_pow10(1), 10);
        assert_eq!(static_pow10(2), 100);
        assert_eq!(static_pow10(3), 1000);
        assert_eq!(static_pow10(4), 10000);
        assert_eq!(static_pow10(5), 100000);
    }
}
