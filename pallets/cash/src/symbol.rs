use crate::types::{Reason, Uint};
use codec::{Decode, Encode};
use our_std::{convert::TryInto, RuntimeDebug};

/// Fixed symbol width.
pub const WIDTH: usize = 12;

/// Type for the abstract symbol of an asset, not tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug)]
pub struct Symbol(pub [char; WIDTH], pub u8);

// Define symbols used directly by the chain itself

/// The non-character value.
pub const NIL: char = 0 as char;

/// Symbol for CASH.
pub const CASH: Symbol = Symbol(
    ['C', 'A', 'S', 'H', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
    6,
);

/// Symbol for USD.
pub const USD: Symbol = Symbol(
    ['U', 'S', 'D', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
    6,
);

/// Get the Uint corresponding to some number of decimals.
///
/// Useful for writing human values in a const context.
pub const fn static_pow10(decimals: u8) -> Uint {
    let mut i = 0;
    let mut v: Uint = 10;
    loop {
        i += 1;
        if i >= decimals {
            return v;
        }
        v *= 10;
    }
}

impl Symbol {
    pub const fn chars(&self) -> &[char] {
        &self.0
    }

    pub const fn decimals(&self) -> u8 {
        self.1
    }

    pub const fn one(&self) -> Uint {
        static_pow10(self.decimals())
    }

    pub fn ticker(&self) -> String {
        let mut s = String::with_capacity(WIDTH);
        let chars = self.chars();
        for i in 0..WIDTH {
            if chars[i] == NIL {
                break;
            }
            s.push(chars[i]);
        }
        s
    }
}

impl Encode for Symbol {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        let mut bytes: Vec<u8> = self.0.to_vec().iter().map(|&c| c as u8).collect();
        bytes.push(self.1);
        bytes.using_encoded(f)
    }
}

impl codec::EncodeLike for Symbol {}

impl Decode for Symbol {
    fn decode<I: codec::Input>(encoded: &mut I) -> Result<Self, codec::Error> {
        let mut bytes: Vec<u8> = Decode::decode(encoded)?;
        let decimals = bytes.pop().unwrap();
        let chars: Vec<char> = bytes.iter().map(|&b| b as char).collect();
        let ticker: [char; WIDTH] = chars.try_into().expect("wrong number of chars");
        Ok(Symbol(ticker, decimals))
    }
}

// Implement deserialization for Symbols so we can use them in GenesisConfig / ChainSpec JSON.
//  i.e. "TICKER" <> ["T", "I", "C", "K", "E", "R", NIL, NIL, NIL, NIL, NIL, NIL]
impl our_std::str::FromStr for Symbol {
    type Err = Reason;

    fn from_str(ticker: &str) -> Result<Self, Self::Err> {
        if let Some((ticker, decimal_str)) = String::from(ticker).split_once("/") {
            let mut chars: Vec<char> = ticker.chars().collect();
            if chars.len() > WIDTH {
                Err(Reason::BadSymbol)
            } else if let Ok(decimals) = decimal_str.parse::<u8>() {
                chars.resize(WIDTH, NIL);
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
