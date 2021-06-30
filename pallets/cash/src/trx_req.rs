use trx_request;

use crate::{
    chains::{ChainAccount, ChainAsset},
    types::CashOrChainAsset,
};

impl From<trx_request::Account> for ChainAccount {
    fn from(account: trx_request::Account) -> Self {
        match account {
            trx_request::Account::Gate(gate_address) => ChainAccount::Gate(gate_address),
            trx_request::Account::Eth(eth_address) => ChainAccount::Eth(eth_address),
        }
    }
}

impl From<trx_request::Asset> for CashOrChainAsset {
    fn from(account: trx_request::Asset) -> Self {
        match account {
            trx_request::Asset::Cash => CashOrChainAsset::Cash,
            trx_request::Asset::Eth(eth_address) => {
                CashOrChainAsset::ChainAsset(ChainAsset::Eth(eth_address))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trx_request;

    const ALAN: [u8; 20] = [1; 20];
    const ETH: [u8; 20] = [238; 20];

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
            CashOrChainAsset::from(trx_request::Asset::Eth(ETH)),
            CashOrChainAsset::ChainAsset(ChainAsset::Eth(ETH))
        );

        assert_eq!(
            CashOrChainAsset::from(trx_request::Asset::Cash),
            CashOrChainAsset::Cash
        );
    }
}
