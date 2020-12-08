use codec::{alloc::vec::Vec, Decode, Encode};

#[derive(Clone, Eq, PartialEq, Debug, Encode, Decode)]
pub enum ChainIdent {
  Eth,
}

/// The type of the decimal field.
pub type AccountAddr = Vec<u8>;

#[derive(Clone, Eq, PartialEq, Debug, Encode, Decode)]
pub struct AccountIdent {
  pub chain: ChainIdent,
  pub account: AccountAddr,
}

impl AccountIdent {
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
