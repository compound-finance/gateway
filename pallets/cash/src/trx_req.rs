extern crate trx_request; // XXX why extern not use?

use crate::chains::ChainAccount;
use crate::types::{Maxable, MaxableAssetAmount};
use trx_request::*;

// XXX why are these the only things needed here?
//  kind of hidden conversion
//   can trx_request return real types?
impl From<trx_request::MaxAmount> for MaxableAssetAmount {
    fn from(amount: MaxAmount) -> Self {
        match amount {
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
    use crate::types::*;

    #[test]
    fn test_max_amount_to_generic() {
        assert_eq!(Maxable::from(MaxAmount::Amt(5)), Maxable::Value(5));
        assert_eq!(Maxable::<AssetAmount>::from(MaxAmount::Max), Maxable::Max);
    }
}
