use codec::{Decode, Encode};
use sp_std::vec::Vec;

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

impl AccountIdent{
  /// Create a new FixedPrecision number from parts. The mantissa is used "raw" and not scaled
  /// in any way
  pub fn new<T: Into<ChainIdent>, D: Into<AccountAddr>>(chain_ident: T, account_addr: D) -> Self {
      AccountIdent {
        chain: chain_ident.into(),
        account: account_addr.into(),
      }
  }
}
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_nothing() {}
}
