extern crate trx_request;

use crate::chains::*;
use crate::core::*;
use trx_request::*;

pub fn max_amount_to_generic(max_amount: MaxAmount, if_max: &Fn() -> GenericQty) -> GenericQty {
    match max_amount {
        MaxAmount::Amt(amt) => amt,
        MaxAmount::Max => if_max(),
    }
}

pub fn account_to_generic(account: trx_request::Account) -> GenericAccount {
    match account {
        trx_request::Account::Eth(address) => (ChainId::Eth, address.into()),
    }
}

// pub fn generic_sig_to_account() -> GenericAccount {s
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_amount_to_generic() {
        assert_eq!(max_amount_to_generic(MaxAmount::Amt(5), &|| 88), 5);
        assert_eq!(max_amount_to_generic(MaxAmount::Max, &|| 88), 88);
    }

    #[test]
    fn test_account_to_generic() {
        assert_eq!(
            account_to_generic(Account::Eth([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20
            ])),
            (
                ChainId::Eth,
                vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]
            )
        );
    }
}
