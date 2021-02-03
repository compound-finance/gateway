use crate::chains::{ChainAccount, ChainAsset};
use trx_request;

impl From<trx_request::Account> for ChainAccount {
    fn from(account: trx_request::Account) -> Self {
        match account {
            trx_request::Account::Eth(eth_address) => ChainAccount::Eth(eth_address),
        }
    }
}

impl From<trx_request::Asset> for ChainAsset {
    fn from(account: trx_request::Asset) -> Self {
        match account {
            trx_request::Asset::Eth(eth_address) => ChainAsset::Eth(eth_address),
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
            ChainAsset::from(trx_request::Asset::Eth(ETH)),
            ChainAsset::Eth(ETH)
        );
    }
}
