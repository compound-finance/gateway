use frame_support::storage::{StorageMap, StorageValue};

#[cfg(test)]
use crate::core::get_asset;
#[cfg(not(test))]
use crate::core::{
    extract_cash_principal_internal, extract_internal, get_asset,
    liquidate_cash_collateral_internal, liquidate_cash_principal_internal, liquidate_internal,
    transfer_cash_principal_internal, transfer_internal,
};
use crate::{
    chains::{ChainAccount, ChainAccountSignature},
    log,
    reason::Reason,
    require,
    symbol::CASH,
    types::{CashOrChainAsset, Nonce, Quantity},
    CashPrincipals, Config, GlobalCashIndex, Nonces,
};
#[cfg(test)]
use mocked_core::{
    extract_cash_principal_internal, extract_internal, liquidate_cash_collateral_internal,
    liquidate_cash_principal_internal, liquidate_internal, transfer_cash_principal_internal,
    transfer_internal,
};
use our_std::str;

pub fn prepend_nonce(payload: &Vec<u8>, nonce: Nonce) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    result.extend_from_slice(nonce.to_string().as_bytes());
    result.extend_from_slice(b":");
    result.extend_from_slice(&payload[..]);
    result
}

// XXX 2 entry points?
pub fn exec<T: Config>(
    request: Vec<u8>,
    signature: ChainAccountSignature,
    nonce: Nonce,
) -> Result<(), Reason> {
    log!("exec: {}", nonce);
    let request_str: &str = str::from_utf8(&request[..]).map_err(|_| Reason::InvalidUTF8)?;
    let sender = signature.recover_account(&prepend_nonce(&request, nonce)[..])?;
    exec_trx_request::<T>(request_str, sender, Some(nonce))
}

pub fn is_minimally_valid_trx_request<T: Config>(
    request: Vec<u8>,
    signature: ChainAccountSignature,
    nonce: Nonce,
) -> Result<(ChainAccount, Nonce), Reason> {
    // Basic request validity checks - valid symbols and parsable request
    let request_str: &str = str::from_utf8(&request[..]).map_err(|_| Reason::InvalidUTF8)?;
    trx_request::parse_request(request_str)?;

    // Signature check
    let sender = signature
        .recover_account(&prepend_nonce(&request, nonce)[..])
        .map_err(|_| Reason::SignatureAccountMismatch)?;

    let current_nonce = Nonces::get(sender);
    Ok((sender, current_nonce))
}

pub fn exec_trx_request<T: Config>(
    request_str: &str,
    sender: ChainAccount,
    nonce_opt: Option<Nonce>,
) -> Result<(), Reason> {
    // Match TrxReq against known Transaction Requests
    let trx_request = trx_request::parse_request(request_str)?;

    if let Some(nonce) = nonce_opt {
        // Read Require Nonce=Nonce_Account+1
        let current_nonce = Nonces::get(sender);
        require!(
            nonce == current_nonce,
            Reason::IncorrectNonce(nonce, current_nonce)
        );
    }

    // XXX still controversial as we read from storage sometimes redundantly,
    //  and calculate amount from principal provided
    //  I think its ok as we should be able to cache things extremely well,
    //   effectively avoiding redundant reads
    //   and its safer to use the principal -> amount than the user amount
    match trx_request {
        trx_request::TrxRequest::Extract(max_amount, asset, account) => {
            match CashOrChainAsset::from(asset) {
                CashOrChainAsset::Cash => match max_amount {
                    trx_request::MaxAmount::Max => {
                        let principal_amount = CashPrincipals::get(sender).amount_withdrawable()?;
                        extract_cash_principal_internal::<T>(
                            sender,
                            account.into(),
                            principal_amount,
                        )?;
                    }

                    trx_request::MaxAmount::Amount(amount) => {
                        let index = GlobalCashIndex::get();
                        let principal_amount =
                            index.cash_principal_amount(Quantity::new(amount, CASH))?;
                        extract_cash_principal_internal::<T>(
                            sender,
                            account.into(),
                            principal_amount,
                        )?;
                    }
                },

                CashOrChainAsset::ChainAsset(chain_asset) => match max_amount {
                    trx_request::MaxAmount::Max => {
                        return Err(Reason::MaxForNonCashAsset);
                    }
                    trx_request::MaxAmount::Amount(amount) => {
                        let asset = get_asset::<T>(chain_asset)?;
                        let asset_amount = asset.as_quantity(amount.into());
                        extract_internal::<T>(asset, sender, account.into(), asset_amount)?;
                    }
                },
            }
        }

        trx_request::TrxRequest::Transfer(max_amount, asset, account) => {
            match CashOrChainAsset::from(asset) {
                CashOrChainAsset::Cash => match max_amount {
                    trx_request::MaxAmount::Max => {
                        let principal_amount = CashPrincipals::get(sender).amount_withdrawable()?;
                        transfer_cash_principal_internal::<T>(
                            sender,
                            account.into(),
                            principal_amount,
                        )?;
                    }

                    trx_request::MaxAmount::Amount(amount) => {
                        let index = GlobalCashIndex::get();
                        let principal_amount =
                            index.cash_principal_amount(Quantity::new(amount, CASH))?;
                        transfer_cash_principal_internal::<T>(
                            sender,
                            account.into(),
                            principal_amount,
                        )?;
                    }
                },

                CashOrChainAsset::ChainAsset(chain_asset) => match max_amount {
                    trx_request::MaxAmount::Max => {
                        return Err(Reason::MaxForNonCashAsset);
                    }

                    trx_request::MaxAmount::Amount(amount) => {
                        let asset = get_asset::<T>(chain_asset)?;
                        let asset_amount = asset.as_quantity(amount.into());
                        transfer_internal::<T>(asset, sender, account.into(), asset_amount)?;
                    }
                },
            }
        }

        trx_request::TrxRequest::Liquidate(
            max_amount,
            trx_borrowed_asset,
            trx_collateral_asset,
            borrower,
        ) => match (
            CashOrChainAsset::from(trx_borrowed_asset),
            CashOrChainAsset::from(trx_collateral_asset),
        ) {
            (x, y) if x == y => Err(Reason::InKindLiquidation),
            (CashOrChainAsset::Cash, CashOrChainAsset::ChainAsset(collateral)) => {
                let collateral_asset = get_asset::<T>(collateral)?;
                let cash_principal_amount = match max_amount {
                    trx_request::MaxAmount::Max => panic!("Not supported"), // TODO
                    trx_request::MaxAmount::Amount(amount) => {
                        let index = GlobalCashIndex::get();
                        index.cash_principal_amount(Quantity::new(amount, CASH))?
                    }
                };

                liquidate_cash_principal_internal::<T>(
                    collateral_asset,
                    sender,
                    borrower.into(),
                    cash_principal_amount,
                )
            }
            (CashOrChainAsset::ChainAsset(borrowed), CashOrChainAsset::Cash) => {
                let borrowed_asset = get_asset::<T>(borrowed)?;
                let borrowed_asset_amount = match max_amount {
                    trx_request::MaxAmount::Max => panic!("Not supported"), // TODO
                    trx_request::MaxAmount::Amount(amount) => {
                        borrowed_asset.as_quantity(amount.into())
                    }
                };

                liquidate_cash_collateral_internal::<T>(
                    borrowed_asset,
                    sender,
                    borrower.into(),
                    borrowed_asset_amount,
                )
            }

            (CashOrChainAsset::ChainAsset(borrowed), CashOrChainAsset::ChainAsset(collateral)) => {
                let borrowed_asset = get_asset::<T>(borrowed)?;
                let collateral_asset = get_asset::<T>(collateral)?;
                let borrowed_asset_amount = match max_amount {
                    trx_request::MaxAmount::Max => panic!("Not supported"), // TODO
                    trx_request::MaxAmount::Amount(amount) => {
                        borrowed_asset.as_quantity(amount.into())
                    }
                };

                liquidate_internal::<T>(
                    borrowed_asset,
                    collateral_asset,
                    sender,
                    borrower.into(),
                    borrowed_asset_amount,
                )
            }
            _ => Err(Reason::InvalidLiquidation), // Probably isn't possible
        }?,
    }

    if let Some(nonce) = nonce_opt {
        // Update user nonce
        Nonces::insert(sender, nonce + 1);
    }

    Ok(())
}

#[allow(dead_code)]
mod mocked_core {
    pub use our_std::result::Result;

    use crate::{
        chains::{ChainAccount, ChainAsset},
        reason::Reason,
        types::{AssetInfo, AssetQuantity, CashPrincipalAmount},
        Config,
    };

    static mut LATEST_CALL: &str = "";

    pub fn extract_internal<T: Config>(
        asset: AssetInfo,
        holder: ChainAccount,
        recipient: ChainAccount,
        amount: AssetQuantity,
    ) -> Result<(), Reason> {
        let latest_call_str = format!(
            "extract_internal: {:?}, {:?}, {:?}, {:?}",
            ChainAsset::from(asset.asset),
            String::from(holder),
            String::from(recipient),
            amount.value
        );
        unsafe {
            LATEST_CALL = Box::leak(latest_call_str.into_boxed_str());
        }

        Ok(())
    }

    pub fn extract_cash_principal_internal<T: Config>(
        holder: ChainAccount,
        recipient: ChainAccount,
        principal: CashPrincipalAmount,
    ) -> Result<(), Reason> {
        let latest_call_str = format!(
            "extract_cash_principal_internal: {:?}, {:?}, {:?}",
            String::from(holder),
            String::from(recipient),
            principal.0
        );
        unsafe {
            LATEST_CALL = Box::leak(latest_call_str.into_boxed_str());
        }

        Ok(())
    }

    pub fn transfer_internal<T: Config>(
        asset: AssetInfo,
        sender: ChainAccount,
        recipient: ChainAccount,
        amount: AssetQuantity,
    ) -> Result<(), Reason> {
        let latest_call_str = format!(
            "transfer_internal: {:?}, {:?}, {:?}, {:?}",
            ChainAsset::from(asset.asset),
            String::from(sender),
            String::from(recipient),
            amount.value
        );
        unsafe {
            LATEST_CALL = Box::leak(latest_call_str.into_boxed_str());
        }

        Ok(())
    }

    pub fn transfer_cash_principal_internal<T: Config>(
        sender: ChainAccount,
        recipient: ChainAccount,
        principal: CashPrincipalAmount,
    ) -> Result<(), Reason> {
        let latest_call_str = format!(
            "transfer_cash_principal_internal: {:?}, {:?}, {:?}",
            String::from(sender),
            String::from(recipient),
            principal.0
        );
        unsafe {
            LATEST_CALL = Box::leak(latest_call_str.into_boxed_str());
        }

        Ok(())
    }

    pub fn liquidate_internal<T: Config>(
        asset: AssetInfo,
        collateral_asset: AssetInfo,
        liquidator: ChainAccount,
        borrower: ChainAccount,
        amount: AssetQuantity,
    ) -> Result<(), Reason> {
        let latest_call_str = format!(
            "liquidate_internal: {:?}, {:?}, {:?}, {:?}, {:?}",
            ChainAsset::from(asset.asset),
            ChainAsset::from(collateral_asset.asset),
            String::from(liquidator),
            String::from(borrower),
            amount.value
        );
        unsafe {
            LATEST_CALL = Box::leak(latest_call_str.into_boxed_str());
        }

        Ok(())
    }

    pub fn liquidate_cash_principal_internal<T: Config>(
        collateral_asset: AssetInfo,
        liquidator: ChainAccount,
        borrower: ChainAccount,
        principal: CashPrincipalAmount,
    ) -> Result<(), Reason> {
        let latest_call_str = format!(
            "liquidate_cash_principal_internal: {:?}, {:?}, {:?}, {:?}",
            ChainAsset::from(collateral_asset.asset),
            String::from(liquidator),
            String::from(borrower),
            principal.0
        );
        unsafe {
            LATEST_CALL = Box::leak(latest_call_str.into_boxed_str());
        }

        Ok(())
    }

    pub fn liquidate_cash_collateral_internal<T: Config>(
        asset: AssetInfo,
        liquidator: ChainAccount,
        borrower: ChainAccount,
        amount: AssetQuantity,
    ) -> Result<(), Reason> {
        let latest_call_str = format!(
            "liquidate_cash_collateral_internal: {:?}, {:?}, {:?}, {:?}",
            ChainAsset::from(asset.asset),
            String::from(liquidator),
            String::from(borrower),
            amount.value
        );
        unsafe {
            LATEST_CALL = Box::leak(latest_call_str.into_boxed_str());
        }

        Ok(())
    }

    pub fn get_latest_call_result() -> String {
        unsafe {
            return LATEST_CALL.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::*, factor::*, rates::*, reason::TrxReqParseError, tests::mock::*, tests::*,
        types::*, *,
    };
    use mocked_core::get_latest_call_result;
    use serial_test::serial;

    fn init_eth_asset() -> Result<(), Reason> {
        let kink_rate = 105;
        let asset = Eth;
        let asset_info = AssetInfo {
            rate_model: InterestRateModel::new_kink(0, kink_rate, Factor::from_nominal("0.5"), 202),
            miner_shares: MinerShares::from_nominal("0.5"),
            ..AssetInfo::minimal(asset, ETH)
        };

        SupportedAssets::insert(&asset, asset_info);

        Ok(())
    }

    fn init_bat_asset() -> Result<(), Reason> {
        const BAT: Units = Units::from_ticker_str("BAT", 18);
        let asset = ChainAsset::from_str("Eth:0x0d8775f648430679a709e98d2b0cb6250d2887ef")?;
        let asset_info = AssetInfo {
            liquidity_factor: LiquidityFactor::from_nominal("0.6543"),
            ..AssetInfo::minimal(asset, BAT)
        };

        SupportedAssets::insert(&asset, asset_info);

        Ok(())
    }

    #[test]
    #[serial]
    fn exec_trx_request_extract_cash_principal_internal() {
        new_test_ext().execute_with(|| {
            let req_str = "(Extract 3 CASH Eth:0x0101010101010101010101010101010101010101)";
            let account = ChainAccount::Eth([20; 20]);
            let nonce = Some(0);

            assert_ok!(exec_trx_request::<Test>(req_str, account, nonce));
            let actual = get_latest_call_result();
            let expected = "extract_cash_principal_internal: \"ETH:0x1414141414141414141414141414141414141414\", \
            \"ETH:0x0101010101010101010101010101010101010101\", 3";
            assert_eq!(actual, expected);
        });
    }

    #[test]
    #[serial]
    fn exec_trx_request_extract_internal() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());

            let req_str = "(Extract 3 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee \
                Eth:0x0101010101010101010101010101010101010101)";
            let account = ChainAccount::Eth([20; 20]);
            let nonce = Some(0);

            assert_ok!(exec_trx_request::<Test>(req_str, account, nonce));
            let actual = get_latest_call_result();
            let expected = "extract_internal: Eth([238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238]), \
            \"ETH:0x1414141414141414141414141414141414141414\", \
            \"ETH:0x0101010101010101010101010101010101010101\", 3";
            assert_eq!(actual, expected);
        });
    }

    #[test]
    #[serial]
    fn exec_trx_transfer_internal() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());

            let req_str = "(Transfer 3 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee \
                Eth:0x0101010101010101010101010101010101010101)";
            let account = ChainAccount::Eth([20; 20]);
            let nonce = Some(0);

            assert_ok!(exec_trx_request::<Test>(req_str, account, nonce));
            let actual = get_latest_call_result();
            let expected = "transfer_internal: Eth([238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238]), \
            \"ETH:0x1414141414141414141414141414141414141414\", \
            \"ETH:0x0101010101010101010101010101010101010101\", 3";
            assert_eq!(actual, expected);
        });
    }

    #[test]
    #[serial]
    fn exec_trx_transfer_principal_cash_internal() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());

            let req_str = "(Transfer 3 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee \
                Eth:0x0101010101010101010101010101010101010101)";
            let account = ChainAccount::Eth([20; 20]);
            let nonce = Some(0);

            assert_ok!(exec_trx_request::<Test>(req_str, account, nonce));
            let actual = get_latest_call_result();
            let expected = "transfer_internal: Eth([238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238]), \
            \"ETH:0x1414141414141414141414141414141414141414\", \
            \"ETH:0x0101010101010101010101010101010101010101\", 3";
            assert_eq!(actual, expected);
        });
    }

    #[test]
    #[serial]
    fn exec_trx_liquidate_cash_collateral_internal() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());

            let req_str = "(Liquidate 55 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee \
                Cash Eth:0x0101010101010101010101010101010101010101)";
            let account = ChainAccount::Eth([20; 20]);
            let nonce = Some(0);

            assert_ok!(exec_trx_request::<Test>(req_str, account, nonce));
            let actual = get_latest_call_result();
            let expected = "liquidate_cash_collateral_internal: Eth([238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238]), \
            \"ETH:0x1414141414141414141414141414141414141414\", \
            \"ETH:0x0101010101010101010101010101010101010101\", 55";
            assert_eq!(actual, expected);
        });
    }

    #[test]
    #[serial]
    fn exec_trx_liquidate_internal() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());
            assert_ok!(init_bat_asset());

            let req_str = "(Liquidate 55 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0d8775f648430679a709e98d2b0cb6250d2887ef Eth:0x0101010101010101010101010101010101010101)";
            let account = ChainAccount::Eth([20; 20]);
            let nonce = Some(0);

            assert_ok!(exec_trx_request::<Test>(req_str, account, nonce));
            let actual = get_latest_call_result();
            let expected = "liquidate_internal: Eth([238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238, 238]), \
            Eth([13, 135, 117, 246, 72, 67, 6, 121, 167, 9, 233, 141, 43, 12, 182, 37, 13, 40, 135, 239]), \
            \"ETH:0x1414141414141414141414141414141414141414\", \"ETH:0x0101010101010101010101010101010101010101\", 55";
            assert_eq!(actual, expected);
        });
    }

    #[test]
    #[serial]
    fn exec_trx_in_kind_liquidation() {
        new_test_ext().execute_with(|| {
            assert_ok!(init_eth_asset());

            let req_str = "(Liquidate 55 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee \
                Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee \
                Eth:0x0101010101010101010101010101010101010101)";
            let account = ChainAccount::Eth([20; 20]);
            let nonce = Some(0);

            assert_eq!(
                exec_trx_request::<Test>(req_str, account, nonce),
                Err(Reason::InKindLiquidation)
            );
        });
    }

    #[test]
    #[serial]
    fn exec_trx_request_wrong_nonce() {
        new_test_ext().execute_with(|| {
            let req_str = "(Liquidate 55 Eth:0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee Eth:0x0d8775f648430679a709e98d2b0cb6250d2887ef Eth:0x0101010101010101010101010101010101010101)";
            let account = ChainAccount::Eth([20; 20]);
            let nonce = Some(3);

            Nonces::insert(account, 2);

            assert_eq!(exec_trx_request::<Test>(req_str, account, nonce), Err(Reason::IncorrectNonce(3, 2)));
        });
    }

    #[test]
    #[serial]
    fn exec_trx_request_invalid_request() {
        new_test_ext().execute_with(|| {
            let req_str = "(INVALID_REQUEST)";
            let account = ChainAccount::Eth([20; 20]);
            let nonce = Some(3);

            assert_eq!(
                exec_trx_request::<Test>(req_str, account, nonce),
                Err(Reason::TrxRequestParseError(TrxReqParseError::LexError))
            );
        });
    }
}
