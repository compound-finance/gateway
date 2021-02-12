use crate::chains::{CashAsset, ChainAccount, ChainAsset};
use trx_request;

impl From<trx_request::Account> for ChainAccount {
    fn from(account: trx_request::Account) -> Self {
        match account {
            trx_request::Account::Eth(eth_address) => ChainAccount::Eth(eth_address),
        }
    }
}

impl From<trx_request::Asset> for CashAsset {
    fn from(account: trx_request::Asset) -> Self {
        match account {
            trx_request::Asset::Cash => CashAsset::Cash,
            trx_request::Asset::Eth(eth_address) => CashAsset::Asset(ChainAsset::Eth(eth_address)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trx_request;

    const ALAN: [u8; 20] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
    const ETH: [u8; 20] = [
        238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238,
        238, 238,
    ];

    #[test]
    fn test_account_to_chain_account() {
        assert_eq!(
            ChainAccount::from(trx_request::Account::Eth(ALAN)),
            ChainAccount::Eth(ALAN)
        );
    }

    #[test]
    fn test_asset_to_chain_asset() {
        assert_eq!(
            CashAsset::from(trx_request::Asset::Eth(ETH)),
            CashAsset::Asset(ChainAsset::Eth(ETH))
        );

        assert_eq!(CashAsset::from(trx_request::Asset::Cash), CashAsset::Cash);
    }
}
