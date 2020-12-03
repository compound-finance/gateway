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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_nothing() {}
}
