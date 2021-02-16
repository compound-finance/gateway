use crate::{
    chains::{CashAsset, ChainAccount},
    core,
    core::symbol,
    reason,
    reason::Reason,
    require,
    symbol::CASH,
    types::{Nonce, Quantity},
    Config, GlobalCashIndex, Nonces,
};
use either::Left;
use frame_support::storage::{StorageMap, StorageValue};

pub fn exec_trx_request<T: Config>(
    request_str: &str,
    sender: ChainAccount,
    nonce_opt: Option<Nonce>,
) -> Result<(), Reason> {
    // Match TrxReq against known Transaction Requests
    let trx_request = trx_request::parse_request(request_str)
        .map_err(|e| Reason::TrxRequestParseError(reason::to_parse_error(e)))?;

    if let Some(nonce) = nonce_opt {
        // Read Require Nonce=Nonce_Account+1
        let current_nonce = Nonces::get(sender);
        require!(
            nonce == current_nonce,
            Reason::IncorrectNonce(nonce, current_nonce)
        );
    }

    match trx_request {
        trx_request::TrxRequest::Extract(amount, asset, account) => match CashAsset::from(asset) {
            CashAsset::Cash => {
                core::extract_cash_internal::<T>(
                    sender,
                    account.into(),
                    Left(Quantity(CASH, amount)),
                )?;
            }
            CashAsset::Asset(chain_asset) => {
                let symbol = symbol(&chain_asset).ok_or(Reason::AssetNotSupported)?;
                let quantity = Quantity(symbol, amount.into());

                core::extract_internal::<T>(chain_asset, sender, account.into(), quantity)?;
            }
        },
        trx_request::TrxRequest::Transfer(amount, asset, account) => match CashAsset::from(asset) {
            CashAsset::Cash => {
                let index = GlobalCashIndex::get(); // XXX This is re-loaded in `transfer_cash_principal_internal`
                let principal = index.as_hold_principal(Quantity(CASH, amount))?; // XXX We later re-calcuate the amount
                core::transfer_cash_principal_internal::<T>(sender, account.into(), principal)?;
            }
            CashAsset::Asset(chain_asset) => {
                let symbol = symbol(&chain_asset).ok_or(Reason::AssetNotSupported)?;
                let quantity = Quantity(symbol, amount.into());

                core::transfer_internal::<T>(chain_asset, sender, account.into(), quantity)?;
            }
        },
    }

    if let Some(nonce) = nonce_opt {
        // Update user nonce
        Nonces::insert(sender, nonce + 1);
    }

    Ok(())
}
