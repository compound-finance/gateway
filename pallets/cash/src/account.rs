use codec::{Decode, Encode};
use our_std::{vec::Vec, RuntimeDebug};

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChainIdent {
    Eth,
}

/// The type of the decimal field.
pub type AccountAddr = Vec<u8>;

// Note: serializable b/c key in storage map.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct AccountIdent {
    pub chain: ChainIdent,
    pub address: AccountAddr,
}

impl AccountIdent {
    pub fn new<T: Into<ChainIdent>, D: Into<AccountAddr>>(chain: T, address: D) -> Self {
        AccountIdent {
            chain: chain.into(),
            address: address.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nothing() {}
}
