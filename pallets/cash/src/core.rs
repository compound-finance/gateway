use crate::{
    chains::{
        self, Chain, ChainAccount, ChainAsset, ChainBlock, ChainBlockEvent, ChainBlockEvents,
        ChainHash, ChainId, ChainSignature, Ethereum,
    },
    internal, log, pipeline,
    portfolio::Portfolio,
    rates::APR,
    reason::Reason,
    types::{
        AssetAmount, AssetBalance, Balance, CashPrincipalAmount, GovernanceResult, NoticeId,
        SignersSet, Timestamp, ValidatorKeys,
    },
    AssetBalances, CashIndex, CashPrincipals, CashYield, Config, Event, GlobalCashIndex,
    IngressionQueue, LastProcessedBlock, Pallet, Starports, SupportedAssets, TotalBorrowAssets,
    TotalCashPrincipal, TotalSupplyAssets, Validators,
};

use codec::Decode;
use frame_support::{
    storage::{IterableStorageMap, StorageDoubleMap, StorageMap, StorageValue},
    traits::UnfilteredDispatchable,
};

#[cfg(any(not(feature = "freeze-time"), not(feature = "std")))]
use sp_runtime::traits::Convert;

pub use our_std::{fmt, result};

// Public helper functions //

/// Return the recent timestamp (from the FREEZE_TIME file).
#[cfg(all(feature = "freeze-time", feature = "std"))]
pub fn get_recent_timestamp<T: Config>() -> Result<Timestamp, Reason> {
    use std::{env, fs};
    if let Ok(filename) = env::var("FREEZE_TIME") {
        if let Ok(contents) = fs::read_to_string(filename) {
            if let Ok(time) = contents.parse::<u64>() {
                println!("Freeze Time: {}", time);
                if time > 0 {
                    return Ok(time);
                }
            }
        }
    }
    return Err(Reason::TimestampMissing);
}

/// Return the recent timestamp (from the timestamp pallet).
#[cfg(not(all(feature = "freeze-time", feature = "std")))]
pub fn get_recent_timestamp<T: Config>() -> Result<Timestamp, Reason> {
    let ts = <pallet_timestamp::Pallet<T>>::get();
    let time = T::TimeConverter::convert(ts);
    if time > 0 {
        return Ok(time);
    }
    return Err(Reason::TimestampMissing);
}

/// Return the event ingression queue for the underlying chain.
pub fn get_event_queue<T: Config>(chain_id: ChainId) -> Result<ChainBlockEvents, Reason> {
    Ok(IngressionQueue::get(chain_id).unwrap_or(ChainBlockEvents::empty(chain_id)?))
}

/// Return the last processed block for the underlying chain, or the initial one for the starport.
pub fn get_last_block<T: Config>(chain_id: ChainId) -> Result<ChainBlock, Reason> {
    LastProcessedBlock::get(chain_id).ok_or(Reason::MissingLastBlock)
}

/// Return the current total borrow and total supply balances for the asset.
pub fn get_market_totals<T: Config>(
    asset: ChainAsset,
) -> Result<(AssetAmount, AssetAmount), Reason> {
    let _info = SupportedAssets::get(asset).ok_or(Reason::AssetNotSupported)?;
    let total_borrow = TotalBorrowAssets::get(asset);
    let total_supply = TotalSupplyAssets::get(asset);
    Ok((total_borrow, total_supply))
}

/// Return the account's balance for the asset.
pub fn get_account_balance<T: Config>(
    account: ChainAccount,
    asset: ChainAsset,
) -> Result<AssetBalance, Reason> {
    let _info = SupportedAssets::get(asset).ok_or(Reason::AssetNotSupported)?;
    Ok(AssetBalances::get(asset, account))
}

/// Return the current cash yield.
pub fn get_cash_yield<T: Config>() -> Result<APR, Reason> {
    Ok(CashYield::get())
}

/// Return the cash total supply data.
pub fn get_cash_data<T: Config>() -> Result<(CashIndex, CashPrincipalAmount), Reason> {
    Ok((GlobalCashIndex::get(), TotalCashPrincipal::get()))
}

/// Return the current borrow and supply rates for the asset.
pub fn get_accounts<T: Config>() -> Result<Vec<ChainAccount>, Reason> {
    let info: Vec<ChainAccount> = CashPrincipals::iter()
        .map(|p| p.0)
        .collect::<Vec<ChainAccount>>();
    Ok(info)
}

/// Return the current borrow and supply rates for the asset.
pub fn get_accounts_liquidity<T: Config>() -> Result<Vec<(ChainAccount, AssetBalance)>, Reason> {
    let mut info: Vec<(ChainAccount, AssetBalance)> = CashPrincipals::iter()
        .map(|a| (a.0.clone(), get_liquidity::<T>(a.0).unwrap().value))
        .collect::<Vec<(ChainAccount, AssetBalance)>>();
    info.sort_by(|(_a_account, a_balance), (_b_account, b_balance)| a_balance.cmp(b_balance));
    Ok(info)
}

/// Calculates the current total CASH value of the account, including all interest from non-CASH markets.
pub fn get_cash_balance_with_asset_interest<T: Config>(
    account: ChainAccount,
) -> Result<Balance, Reason> {
    Ok(pipeline::load_portfolio::<T>(account)?.cash)
}

/// Return the portfolio of the chain account.
pub fn get_portfolio<T: Config>(account: ChainAccount) -> Result<Portfolio, Reason> {
    Ok(pipeline::load_portfolio::<T>(account)?)
}

/// Calculates the current liquidity value for an account.
pub fn get_liquidity<T: Config>(account: ChainAccount) -> Result<Balance, Reason> {
    Ok(pipeline::load_portfolio::<T>(account)?.get_liquidity::<T>()?)
}

/// Return the set of validator identities to compare with others.
pub fn get_validator_set<T: Config>() -> Result<SignersSet, Reason> {
    // Note: inefficient, probably manage reading validators from storage better
    Ok(Validators::iter().map(|(_, v)| v.substrate_id).collect())
}

/// Return the validator associated with the given signer account.
pub fn get_validator<T: Config>(signer: ChainAccount) -> Result<ValidatorKeys, Reason> {
    // Note: inefficient, we should index
    match signer {
        ChainAccount::Eth(eth_address) => {
            for (_, validator) in Validators::iter() {
                if validator.eth_address == eth_address {
                    return Ok(validator);
                }
            }
            return Err(Reason::UnknownValidator);
        }

        _ => {
            // this is a placeholder for future variants, which should be kept minimal
            //  since generally we dont want validators to have to add new types of keys
            return Err(Reason::NotImplemented); // XXX
        }
    }
}

/// Return the validator as seen to be itself by the current worker.
pub fn get_current_validator<T: Config>() -> Result<ValidatorKeys, Reason> {
    // Note: we can lookup *any* signer, may as well choose the first and only option (Eth)
    get_validator::<T>(ChainAccount::Eth(<Ethereum as Chain>::signer_address()?))
}

/// Return the starport associated with a given chain
pub fn get_starport<T: Config>(chain_id: ChainId) -> Result<ChainAccount, Reason> {
    Starports::get(chain_id).ok_or(Reason::StarportMissing)
}

/// Return the validator which signed the given data, given signature.
pub fn recover_validator<T: Config>(
    data: &[u8],
    signature: ChainSignature,
) -> Result<ValidatorKeys, Reason> {
    // Note: inefficient, we should index by every key we want to query by
    match signature {
        ChainSignature::Eth(eth_sig) => {
            let eth_address = <Ethereum as Chain>::recover_address(data, eth_sig)?;
            for (_, validator) in Validators::iter() {
                if validator.eth_address == eth_address {
                    return Ok(validator);
                }
            }
        }

        _ => {
            // this is a placeholder for future variants, which should be kept minimal
            //  since generally we dont want validators to have to add new types of keys
            return Err(Reason::NotImplemented); // XXX
        }
    }

    Err(Reason::UnknownValidator)
}

/// Sign the given data as a validator, assuming we have the credentials.
/// The validator can sign with any valid ChainSignature, which happens to only be Eth currently.
pub fn validator_sign<T: Config>(data: &[u8]) -> Result<ChainSignature, Reason> {
    Ok(ChainSignature::Eth(<Ethereum as Chain>::sign_message(
        data,
    )?))
}

// Protocol interface //

/// Apply the event to the current state, effectively taking the action.
pub fn apply_chain_event_internal<T: Config>(event: &ChainBlockEvent) -> Result<(), Reason> {
    log!("apply_chain_event_internal(event): {:?}", event);

    match event {
        ChainBlockEvent::Eth(_block_num, eth_event) => match eth_event {
            ethereum_client::EthereumEvent::Lock {
                asset,
                sender,
                chain,
                recipient,
                amount,
            } => internal::lock::lock_internal::<T>(
                internal::assets::get_asset::<T>(ChainAsset::Eth(*asset))?,
                ChainAccount::Eth(*sender),
                chains::get_chain_account(chain.to_string(), *recipient)?,
                internal::assets::get_quantity::<T>(ChainAsset::Eth(*asset), *amount)?,
            ),

            ethereum_client::EthereumEvent::LockCash {
                sender,
                chain,
                recipient,
                principal,
                ..
            } => internal::lock::lock_cash_principal_internal::<T>(
                ChainAccount::Eth(*sender),
                chains::get_chain_account(chain.to_string(), *recipient)?,
                CashPrincipalAmount(*principal),
            ),

            ethereum_client::EthereumEvent::ExecuteProposal {
                title: _title,
                extrinsics,
            } => dispatch_extrinsics_internal::<T>(extrinsics.to_vec()),

            ethereum_client::EthereumEvent::ExecTrxRequest {
                account,
                trx_request,
            } => internal::exec_trx_request::exec_trx_request::<T>(
                &trx_request[..],
                ChainAccount::Eth(*account),
                None,
            ),

            ethereum_client::EthereumEvent::NoticeInvoked {
                era_id,
                era_index,
                notice_hash,
                result,
            } => internal::notices::handle_notice_invoked::<T>(
                ChainId::Eth,
                NoticeId(*era_id, *era_index),
                ChainHash::Eth(*notice_hash),
                result.to_vec(),
            ),
        },
    }
}

/// Un-apply the event on the current state, undoing the action to the extent possible/necessary.
pub fn unapply_chain_event_internal<T: Config>(event: &ChainBlockEvent) -> Result<(), Reason> {
    log!("unapply_chain_event_internal(event): {:?}", event);

    match event {
        ChainBlockEvent::Eth(_block_num, eth_event) => match eth_event {
            ethereum_client::EthereumEvent::Lock {
                asset,
                sender,
                chain,
                recipient,
                amount,
            } => internal::lock::undo_lock_internal::<T>(
                internal::assets::get_asset::<T>(ChainAsset::Eth(*asset))?,
                ChainAccount::Eth(*sender),
                chains::get_chain_account(chain.to_string(), *recipient)?,
                internal::assets::get_quantity::<T>(ChainAsset::Eth(*asset), *amount)?,
            ),

            ethereum_client::EthereumEvent::LockCash {
                sender,
                chain,
                recipient,
                principal,
                ..
            } => internal::lock::undo_lock_cash_principal_internal::<T>(
                ChainAccount::Eth(*sender),
                chains::get_chain_account(chain.to_string(), *recipient)?,
                CashPrincipalAmount(*principal),
            ),

            _ => Ok(()),
        },
    }
}

pub fn dispatch_extrinsics_internal<T: Config>(extrinsics: Vec<Vec<u8>>) -> Result<(), Reason> {
    // Decode a SCALE-encoded set of extrinsics from the event
    // For each extrinsic, dispatch the given extrinsic as Root
    let results: Vec<(Vec<u8>, GovernanceResult)> = extrinsics
        .into_iter()
        .map(|payload| {
            log!(
                "dispatch_extrinsics_internal:: dispatching extrinsic {}",
                hex::encode(&payload)
            );
            let call_res: Result<<T as Config>::Call, _> = Decode::decode(&mut &payload[..]);
            match call_res {
                Ok(call) => {
                    log!("dispatch_extrinsics_internal:: dispatching {:?}", call);
                    let res = call.dispatch_bypass_filter(frame_system::RawOrigin::Root.into());

                    let gov_res = match res {
                        Ok(_) => GovernanceResult::DispatchSuccess,
                        Err(error_with_post_info) => {
                            GovernanceResult::DispatchFailure(error_with_post_info.error)
                        }
                    };

                    log!("dispatch_extrinsics_internal:: res {:?}", res);
                    (payload, gov_res)
                }
                _ => {
                    log!(
                        "dispatch_extrinsics_internal:: failed to decode extrinsic {}",
                        hex::encode(&payload)
                    );
                    (payload, GovernanceResult::FailedToDecodeCall)
                }
            }
        })
        .collect();

    <Pallet<T>>::deposit_event(Event::ExecutedGovernance(results));

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::factor::Factor;
    use crate::tests::*;

    #[test]
    fn test_compute_cash_principal_per() -> Result<(), Reason> {
        // round numbers (unrealistic but very easy to check)
        let asset_rate = APR::from_nominal("0.30"); // 30% per year
        let dt = MILLISECONDS_PER_YEAR / 2; // for 6 months
        let cash_index = CashIndex::from_nominal("1.5"); // current index value 1.5
        let price_asset = Price::from_nominal(CASH.ticker, "1500"); // $1,500
        let price_cash = Price::from_nominal(CASH.ticker, "1");
        let price_ratio = Factor::ratio(price_asset, price_cash)?;

        let actual = cash_index.cash_principal_per_asset(asset_rate.simple(dt)?, price_ratio)?;
        let expected = CashPrincipalPerAsset::from_nominal("150"); // from hand calc
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_compute_cash_principal_per_specific_case() -> Result<(), Reason> {
        // a unit test related to previous unexpected larger scope test of on_initialize
        // this showed that we should divide by SECONDS_PER_YEAR last te prevent un-necessary truncation
        let asset_rate = APR::from_nominal("0.1225");
        let dt = MILLISECONDS_PER_YEAR / 4;
        let cash_index = CashIndex::from_nominal("1.123");
        let price_asset = Price::from_nominal(CASH.ticker, "1450");
        let price_cash = Price::from_nominal(CASH.ticker, "1");
        let price_ratio = Factor::ratio(price_asset, price_cash)?;

        let actual = cash_index.cash_principal_per_asset(asset_rate.simple(dt)?, price_ratio)?;
        let expected = CashPrincipalPerAsset::from_nominal("39.542520035618878005"); // from hand calc
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_compute_cash_principal_per_realistic_underflow_case() -> Result<(), Reason> {
        // a unit test related to previous unexpected larger scope test of on_initialize
        // This case showed that we should have more decimals on CASH token to avoid 0 interest
        // showing for common cases. We want "number go up" technology.
        let asset_rate = APR::from_nominal("0.156");
        let dt = 6000;
        let cash_index = CashIndex::from_nominal("4.629065392511782467");
        let price_asset = Price::from_nominal(CASH.ticker, "0.313242");
        let price_cash = Price::from_nominal(CASH.ticker, "1");
        let price_ratio = Factor::ratio(price_asset, price_cash)?;

        let actual = cash_index.cash_principal_per_asset(asset_rate.simple(dt)?, price_ratio)?;
        let expected = CashPrincipalPerAsset::from_nominal("0.000000002008426366"); // from hand calc
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_get_recent_timestamp() {
        new_test_ext().execute_with(|| {
            let expected = 123;
            <pallet_timestamp::Pallet<Test>>::set_timestamp(expected);
            let actual = get_recent_timestamp::<Test>().unwrap();
            assert_eq!(actual, expected);
        });
    }

    #[test]
    fn test_get_current_validator() {
        new_test_ext().execute_with(|| {
            let validator = ValidatorKeys {
                substrate_id: AccountId32::new([0u8; 32]),
                eth_address: <Ethereum as Chain>::signer_address().unwrap(),
            };
            Validators::insert(validator.substrate_id.clone(), &validator);
            assert_eq!(get_current_validator::<Test>().unwrap(), validator);
        })
    }
}
