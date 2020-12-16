use codec::{Decode, Encode};
use sp_std::vec::Vec;

use sp_runtime::RuntimeDebug;
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize}; // XXX wrap these

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ChainIdent {
    Eth,
}

/// The type of the decimal field.
pub type AccountAddr = Vec<u8>;

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
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
