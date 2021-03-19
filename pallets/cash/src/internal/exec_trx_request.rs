use frame_support::storage::{StorageMap, StorageValue};

use crate::{
    chains::{ChainAccount, ChainAccountSignature},
    core::{self, get_asset},
    log,
    reason::Reason,
    require,
    symbol::CASH,
    types::{CashOrChainAsset, Nonce, Quantity},
    CashPrincipals, Config, GlobalCashIndex, Nonces,
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

// TODO: Unit tests!!
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
                        core::extract_cash_principal_internal::<T>(
                            sender,
                            account.into(),
                            principal_amount,
                        )?;
                    }

                    trx_request::MaxAmount::Amount(amount) => {
                        let index = GlobalCashIndex::get();
                        let principal_amount =
                            index.cash_principal_amount(Quantity::new(amount, CASH))?;
                        core::extract_cash_principal_internal::<T>(
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
                        core::extract_internal::<T>(asset, sender, account.into(), asset_amount)?;
                    }
                },
            }
        }

        trx_request::TrxRequest::Transfer(max_amount, asset, account) => {
            match CashOrChainAsset::from(asset) {
                CashOrChainAsset::Cash => match max_amount {
                    trx_request::MaxAmount::Max => {
                        let principal_amount = CashPrincipals::get(sender).amount_withdrawable()?;
                        core::transfer_cash_principal_internal::<T>(
                            sender,
                            account.into(),
                            principal_amount,
                        )?;
                    }

                    trx_request::MaxAmount::Amount(amount) => {
                        let index = GlobalCashIndex::get();
                        let principal_amount =
                            index.cash_principal_amount(Quantity::new(amount, CASH))?;
                        core::transfer_cash_principal_internal::<T>(
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
                        core::transfer_internal::<T>(asset, sender, account.into(), asset_amount)?;
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

                core::liquidate_cash_principal_internal::<T>(
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

                core::liquidate_cash_collateral_internal::<T>(
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

                core::liquidate_internal::<T>(
                    borrowed_asset,
                    collateral_asset,
                    sender,
                    borrower.into(),
                    borrowed_asset_amount,
                )
            }
            _ => Err(Reason::InvalidLiquidation), // Probably isn't possible
        }?,
        _ => Err(Reason::InvalidTrxRequest)?,
    }

    if let Some(nonce) = nonce_opt {
        // Update user nonce
        Nonces::insert(sender, nonce + 1);
    }

    Ok(())
}
