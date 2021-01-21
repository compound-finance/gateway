use crate::types::Uint;
use codec::{Decode, Encode};
use our_std::{convert::TryInto, RuntimeDebug};

/// Type for the abstract symbol of an asset, not tied to a chain.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug)]
pub struct Symbol(pub [char; 12], pub u8);

// Define symbols used directly by the chain itself
pub const NIL: char = 0 as char;
pub const CASH: Symbol = Symbol(
    ['C', 'A', 'S', 'H', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
    6,
);
pub const USD: Symbol = Symbol(
    ['U', 'S', 'D', NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL, NIL],
    6,
);

/// Get the Uint corresponding to some number of decimals. Useful for scaling things up and down.
///
/// WARNING - This function is inefficient and unsafe (overflow) and should ONLY be used in const
/// contexts. This function will panic on overflow in a const context but NOT in --release binaries
/// during runtime. Do not use this function in any runtime code (non-const)
///
/// From the rust docs on const evaluation
///
/// > Behaviors such as out of bounds array indexing or overflow are compiler errors if the value
/// > must be evaluated at compile time (i.e. in const contexts). Otherwise, these behaviors are
/// > warnings, but will likely panic at run-time.
///
/// https://doc.rust-lang.org/reference/const_eval.html
const fn pow10(decimals: u8) -> Uint {
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
    pub const fn ticker(&self) -> &[char] {
        &self.0
    }

    pub const fn decimals(&self) -> u8 {
        self.1
    }

    pub const fn one(&self) -> Uint {
        pow10(self.decimals())
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
        let ticker: [char; 12] = chars.try_into().expect("wrong number of chars");
        Ok(Symbol(ticker, decimals))
    }
}
