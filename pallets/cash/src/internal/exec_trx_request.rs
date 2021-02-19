use either::{Left, Right};
use frame_support::storage::{StorageMap, StorageValue};

use crate::{
    chains::{ChainAccount, ChainAccountSignature},
    core::{self, get_asset},
    log,
    reason::{MathError, Reason},
    require,
    symbol::CASH,
    types::{CashOrChainAsset, CashPrincipal, Nonce, Quantity},
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
                                MathError::SignMismatch.into() // XXX unsigned cash principal
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
                        MathError::SignMismatch.into() // XXX unsigned principal
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
