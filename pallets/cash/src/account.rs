use codec::{Decode, Encode};
use sp_std::vec::Vec;

#[derive(Clone, Eq, PartialEq, Debug, Encode, Decode)]
pub enum ChainIdent {
  Eth,
}

// trait Chain {
//   fn hash<ChainIdent>(input: Vec<u8>);
// }

// impl Chain<Eth> for ChainIdent {
//   fn hash(input: Vec<u8>) -> [u8; 32] {
//     return [9];
//   }
// }

/// The type of the decimal field.
pub type AccountAddr = Vec<u8>;

#[derive(Clone, Eq, PartialEq, Debug, Encode, Decode)]
pub struct AccountIdent {
  pub chain: ChainIdent,
  pub account: AccountAddr,
}

// impl Copy for AccountIdent {
//   fn clone(&self) -> AccountIdent {
//       *self
//   }
// }

impl AccountIdent{
  /// Create a new FixedPrecision number from parts. The mantissa is used "raw" and not scaled
  /// in any way
  pub fn new<T: Into<ChainIdent>, D: Into<AccountAddr>>(chainIdent: T, accountAddr: D) -> Self {
      AccountIdent {
        chain: chainIdent.into(),
        account: accountAddr.into(),
      }
  }
}
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_nothing() {}
}
