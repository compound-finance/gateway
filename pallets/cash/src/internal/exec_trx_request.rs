use crate::{
    chains::{CashAsset, ChainAccount},
    core::{extract_cash_internal, extract_internal, symbol},
    reason,
    reason::Reason,
    require,
    symbol::CASH,
    types::{Nonce, Quantity},
    Config, Nonces,
};
use either::Left;
use frame_support::storage::StorageMap;

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
                extract_cash_internal::<T>(sender, account.into(), Left(Quantity(CASH, amount)))?;
            }
            CashAsset::Asset(chain_asset) => {
                let symbol = symbol(&chain_asset).ok_or(Reason::AssetNotSupported)?;
                let quantity = Quantity(symbol, amount.into());

                extract_internal::<T>(chain_asset, sender, account.into(), quantity)?;
            }
        },
    }

    if let Some(nonce) = nonce_opt {
        // Update user nonce
        Nonces::insert(sender, nonce + 1);
    }

    Ok(())
}
