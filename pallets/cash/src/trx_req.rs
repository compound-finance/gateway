extern crate trx_request;

use crate::types::{AssetAmount, ChainAccount, Maxable};
use trx_request::*;

impl From<trx_request::MaxAmount> for Maxable<AssetAmount> {
    fn from(amt: MaxAmount) -> Self {
        match amt {
            MaxAmount::Max => Maxable::Max,
            MaxAmount::Amt(amt) => Maxable::Value(amt),
        }
    }
}

impl From<trx_request::Account> for ChainAccount {
    fn from(account: trx_request::Account) -> Self {
        match account {
            trx_request::Account::Eth(eth_acc) => ChainAccount::Eth(eth_acc),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_amount_to_generic() {
        assert_eq!(Maxable::from(MaxAmount::Amt(5)), Maxable::Value(5));
        assert_eq!(Maxable::from(MaxAmount::Max), Maxable::Max);
    }
}
