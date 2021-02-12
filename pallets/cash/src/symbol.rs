use crate::reason::Reason;
use crate::types::Uint;
use codec::{Decode, Encode};
use our_std::{convert::TryInto, RuntimeDebug};

/// Fixed symbol width.
pub const WIDTH: usize = 12;

/// Type for the abstract symbol of an asset, not tied to a chain.
#[derive(Copy, Clone, Eq, Encode, Decode, PartialEq, Ord, PartialOrd, RuntimeDebug)]
pub struct Symbol(pub [u8; WIDTH], pub u8);

// Define symbols used directly by the chain itself

/// Symbol for CASH.
pub const CASH: Symbol = Symbol::new("CASH", 6);

/// Symbol for USD.
pub const USD: Symbol = Symbol::new("USD", 6);

/// Get the Uint corresponding to some number of decimals.
///
/// Useful for writing human values in a const context.
pub const fn static_pow10(decimals: u8) -> Uint {
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

impl Symbol {
    pub const fn new(ticker: &str, decimals: u8) -> Self {
        assert!(ticker.len() < WIDTH, "Too many chars");
        let mut bytes = [0u8; WIDTH];
        let mut i = 0;
        let raw = ticker.as_bytes();
        loop {
            if i >= ticker.len() {
                break;
            }
            bytes[i] = raw[i];
            i += 1;
        }
        Symbol(bytes, decimals)
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0[..]
    }

    pub const fn decimals(&self) -> u8 {
        self.1
    }

    pub const fn one(&self) -> Uint {
        static_pow10(self.decimals())
    }

    pub fn ticker(&self) -> String {
        let mut s = String::with_capacity(WIDTH);
        let bytes = self.bytes();
        for i in 0..WIDTH {
            if bytes[i] == 0 {
                break;
            }
            s.push(bytes[i] as char);
        }
        s
    }
}

// Implement deserialization for Symbols so we can use them in GenesisConfig / ChainSpec JSON.
//  i.e. "TICKER" <> ["T", "I", "C", "K", "E", "R", 0, 0, 0, 0, 0, 0]
impl our_std::str::FromStr for Symbol {
    type Err = Reason;

    fn from_str(ticker: &str) -> Result<Self, Self::Err> {
        if let Some((ticker, decimal_str)) = String::from(ticker).split_once("/") {
            let mut chars: Vec<u8> = ticker.as_bytes().into();
            if chars.len() > WIDTH {
                Err(Reason::BadSymbol)
            } else if let Ok(decimals) = decimal_str.parse::<u8>() {
                chars.resize(WIDTH, 0);
                Ok(Symbol(chars.try_into().unwrap(), decimals))
            } else {
                Err(Reason::BadSymbol)
            }
        } else {
            Err(Reason::BadSymbol)
        }
    }
}

// For serialize (which we don't use, but are required to implement)
impl From<Symbol> for String {
    fn from(symbol: Symbol) -> String {
        format!("{}/{}", symbol.ticker(), symbol.decimals())
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
