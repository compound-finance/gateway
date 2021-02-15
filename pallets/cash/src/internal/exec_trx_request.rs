use crate::{
    chains::ChainAccount,
    core::{self, get_asset},
    reason::Reason,
    require,
    symbol::CASH,
    types::{CashOrChainAsset, CashPrincipal, Nonce, Quantity},
    CashPrincipals, Config, GlobalCashIndex, Nonces,
};
use either::{Left, Right};
use frame_support::storage::{StorageMap, StorageValue};

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

    match trx_request {
        trx_request::TrxRequest::Extract(max_amount, asset, account) => {
            match CashOrChainAsset::from(asset) {
                CashOrChainAsset::Cash => {
                    let quantity = match max_amount {
                        trx_request::MaxAmount::Max => {
                            // Read Principal=CashPrincipal_Signer assuming CashPrincipal_Signer ≥ 0 or fail
                            let cash_principal_signer = CashPrincipals::get(sender);
                            require!(
                                cash_principal_signer >= CashPrincipal::ZERO,
                                Reason::MaxWithNegativeCashPrincipal
                            );

                            Right(cash_principal_signer)
                        }

                        // XXX Should we check for negative amounts / principals?
                        trx_request::MaxAmount::Amount(amount) => Left(Quantity::new(amount, CASH)),
                    };
                    core::extract_cash_internal::<T>(sender, account.into(), quantity)?;
                }

                CashOrChainAsset::ChainAsset(chain_asset) => match max_amount {
                    trx_request::MaxAmount::Max => {
                        return Err(Reason::MaxForNonCashAsset);
                    }
                    trx_request::MaxAmount::Amount(amount) => {
                        let asset = get_asset::<T>(chain_asset)?;
                        core::extract_internal::<T>(
                            asset,
                            sender,
                            account.into(),
                            asset.as_quantity(amount.into()),
                        )?;
                    }
                },
            }
        }
        trx_request::TrxRequest::Transfer(max_amount, asset, account) => {
            match CashOrChainAsset::from(asset) {
                CashOrChainAsset::Cash => {
                    let index = GlobalCashIndex::get(); // XXX This is re-loaded in `transfer_cash_principal_internal`
                    let principal = match max_amount {
                        trx_request::MaxAmount::Max => CashPrincipals::get(sender),
                        trx_request::MaxAmount::Amount(amount) => {
                            index.as_hold_principal(Quantity::new(amount, CASH))?
                            // XXX We later re-calcuate the amount
                        }
                    };

                    // XXX Where should we do this check
                    require!(
                        principal >= CashPrincipal::ZERO,
                        Reason::MaxWithNegativeCashPrincipal
                    );
                    core::transfer_cash_principal_internal::<T>(sender, account.into(), principal)?;
                }

                CashOrChainAsset::ChainAsset(chain_asset) => match max_amount {
                    trx_request::MaxAmount::Max => {
                        return Err(Reason::MaxForNonCashAsset);
                    }
                    trx_request::MaxAmount::Amount(amount) => {
                        let asset = get_asset::<T>(chain_asset)?;
                        core::transfer_internal::<T>(
                            asset,
                            sender,
                            account.into(),
                            asset.as_quantity(amount.into()),
                        )?;
                    }
                },
            }
        }
    }

    if let Some(nonce) = nonce_opt {
        // Update user nonce
        Nonces::insert(sender, nonce + 1);
    }

    Ok(())
}
