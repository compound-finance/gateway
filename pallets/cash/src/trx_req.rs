extern crate trx_request;

use crate::chains::*;
use crate::core::*;
use trx_request::*;

impl From<trx_request::MaxAmount> for GenericMaxQty {
    fn from(amt: MaxAmount) -> Self {
        match amt {
            MaxAmount::Max => GenericMaxQty::Max,
            MaxAmount::Amt(amt) => GenericMaxQty::Qty(amt),
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
        assert_eq!(
            GenericMaxQty::from(MaxAmount::Amt(5)),
            GenericMaxQty::Qty(5)
        );
        assert_eq!(GenericMaxQty::from(MaxAmount::Max), GenericMaxQty::Max);
    }
}
