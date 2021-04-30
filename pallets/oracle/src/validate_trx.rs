use crate::{error::OracleError, oracle, Call, Config};
use codec::{Decode, Encode};
use our_std::{log, RuntimeDebug};
use sp_runtime::transaction_validity::{TransactionSource, TransactionValidity, ValidTransaction};

const MAX_EXTERNAL_PAIRS: usize = 30;
const UNSIGNED_TXS_PRIORITY: u64 = 100;
const UNSIGNED_TXS_LONGEVITY: u64 = 32;

#[derive(Encode, Decode, Eq, PartialEq, RuntimeDebug, Clone, Copy)]
pub enum ValidationError {
    InvalidInternalOnly,
    InvalidPriceSignature,
    InvalidPrice(OracleError),
    InvalidCall,
    ExcessivePrices,
}

pub fn check_validation_failure<T: Config>(
    call: &Call<T>,
    res: Result<TransactionValidity, ValidationError>,
) -> Result<TransactionValidity, ValidationError> {
    if let Err(err) = res {
        log!("validate_unsigned: call = {:#?}, error = {:#?}", call, err);
    }
    res
}

pub fn validate_unsigned<T: Config>(
    source: TransactionSource,
    call: &Call<T>,
) -> Result<TransactionValidity, ValidationError> {
    match call {
        Call::post_price(payload, signature) => {
            if oracle::check_signature::<T>(&payload, &signature) == Ok(true) {
                match source {
                    TransactionSource::Local | TransactionSource::InBlock => {
                        Ok(ValidTransaction::with_tag_prefix("Gateway::post_price")
                            .priority(UNSIGNED_TXS_PRIORITY)
                            .longevity(UNSIGNED_TXS_LONGEVITY)
                            .and_provides(signature)
                            .propagate(false)
                            .build())
                    }
                    _ => match oracle::get_and_check_parsed_price::<T>(payload) {
                        Ok(_) => Ok(ValidTransaction::with_tag_prefix("Gateway::post_price")
                            .priority(UNSIGNED_TXS_PRIORITY)
                            .longevity(UNSIGNED_TXS_LONGEVITY)
                            .and_provides(signature)
                            .propagate(true)
                            .build()),
                        Err(err) => Err(ValidationError::InvalidPrice(err)),
                    },
                }
            } else {
                Err(ValidationError::InvalidPriceSignature)
            }
        }
        Call::post_prices(pairs) => {
            let signatures: Vec<_> = pairs.iter().map(|(_, s)| s).collect();
            let if_valid = match source {
                TransactionSource::Local | TransactionSource::InBlock => {
                    Ok(ValidTransaction::with_tag_prefix("Gateway::post_prices")
                        .priority(UNSIGNED_TXS_PRIORITY)
                        .longevity(UNSIGNED_TXS_LONGEVITY)
                        .and_provides(signatures)
                        .propagate(false)
                        .build())
                }
                _ => {
                    if pairs.iter().count() < MAX_EXTERNAL_PAIRS {
                        Ok(ValidTransaction::with_tag_prefix("Gateway::post_prices")
                            .priority(UNSIGNED_TXS_PRIORITY)
                            .longevity(UNSIGNED_TXS_LONGEVITY)
                            .and_provides(signatures)
                            .propagate(true)
                            .build())
                    } else {
                        Err(ValidationError::ExcessivePrices)
                    }
                }
            };

            pairs
                .into_iter()
                .fold(if_valid, |acc, (payload, signature)| match acc {
                    Err(err) => Err(err),
                    Ok(validation) => {
                        if oracle::check_signature::<T>(&payload, &signature) == Ok(true) {
                            match source {
                                TransactionSource::Local | TransactionSource::InBlock => {
                                    Ok(validation)
                                }
                                _ => match oracle::get_and_check_parsed_price::<T>(payload) {
                                    Ok(_) => Ok(validation),
                                    Err(err) => Err(ValidationError::InvalidPrice(err)),
                                },
                            }
                        } else {
                            Err(ValidationError::InvalidPriceSignature)
                        }
                    }
                })
        }
        _ => Err(ValidationError::InvalidCall),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tests::*, ticker::Ticker, types::ReporterSet, Call, PriceReporters, PriceTimes};
    use frame_support::storage::{StorageMap, StorageValue};

    #[test]
    fn test_post_price_invalid_signature() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::post_price::<Test>(vec![], vec![]),
                ),
                Err(ValidationError::InvalidPriceSignature)
            );
        });
    }

    #[test]
    fn test_post_price_stale() {
        new_test_ext().execute_with(|| {
            PriceReporters::put(ReporterSet(vec![[133, 97, 91, 7, 102, 21, 49, 124, 128, 241, 76, 186, 214, 80, 30, 236, 3, 28, 213, 28]]));
            let ticker = Ticker::new("BTC");
            PriceTimes::insert(ticker, 999999999999999);
            let msg = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000688e4cda00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000");
            let sig = hex_literal::hex!("69538bfa1a2097ea206780654d7baac3a17ee57547ee3eeb5d8bcb58a2fcdf401ff8834f4a003193f24224437881276fe76c8e1c0a361081de854457d41d0690000000000000000000000000000000000000000000000000000000000000001c");

            assert_eq!(
                validate_unsigned(
                    TransactionSource::External {},
                    &Call::post_price::<Test>(msg.to_vec(), sig.to_vec()),
                ),
                Err(ValidationError::InvalidPrice(OracleError::StalePrice))
            );
        });
    }

    #[test]
    fn test_post_price_valid_remote() {
        new_test_ext().execute_with(|| {
            PriceReporters::put(ReporterSet(vec![[133, 97, 91, 7, 102, 21, 49, 124, 128, 241, 76, 186, 214, 80, 30, 236, 3, 28, 213, 28]]));

            let msg = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000688e4cda00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000");
            let sig = hex_literal::hex!("69538bfa1a2097ea206780654d7baac3a17ee57547ee3eeb5d8bcb58a2fcdf401ff8834f4a003193f24224437881276fe76c8e1c0a361081de854457d41d0690000000000000000000000000000000000000000000000000000000000000001c");

            assert_eq!(
                validate_unsigned(
                    TransactionSource::External {},
                    &Call::post_price::<Test>(msg.to_vec(), sig.to_vec()),
                ),
                Ok(ValidTransaction::with_tag_prefix("Gateway::post_price")
                    .priority(UNSIGNED_TXS_PRIORITY)
                    .longevity(UNSIGNED_TXS_LONGEVITY)
                    .and_provides(sig.to_vec())
                    .propagate(true)
                    .build())
            );
        });
    }

    #[test]
    fn test_post_price_valid_local() {
        new_test_ext().execute_with(|| {
            PriceReporters::put(ReporterSet(vec![[133, 97, 91, 7, 102, 21, 49, 124, 128, 241, 76, 186, 214, 80, 30, 236, 3, 28, 213, 28]]));

            let msg = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000688e4cda00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000");
            let sig = hex_literal::hex!("69538bfa1a2097ea206780654d7baac3a17ee57547ee3eeb5d8bcb58a2fcdf401ff8834f4a003193f24224437881276fe76c8e1c0a361081de854457d41d0690000000000000000000000000000000000000000000000000000000000000001c");

            assert_eq!(
                validate_unsigned(
                    TransactionSource::Local {},
                    &Call::post_price::<Test>(msg.to_vec(), sig.to_vec()),
                ),
                Ok(ValidTransaction::with_tag_prefix("Gateway::post_price")
                    .priority(UNSIGNED_TXS_PRIORITY)
                    .longevity(UNSIGNED_TXS_LONGEVITY)
                    .and_provides(sig.to_vec())
                    .propagate(false)
                    .build())
            );
        });
    }

    #[test]
    fn test_post_prices_invalid_signature() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::post_prices::<Test>(vec![(vec![], vec![])]),
                ),
                Err(ValidationError::InvalidPriceSignature)
            );
        });
    }

    #[test]
    fn test_post_prices_stale() {
        new_test_ext().execute_with(|| {
            PriceReporters::put(ReporterSet(vec![[133, 97, 91, 7, 102, 21, 49, 124, 128, 241, 76, 186, 214, 80, 30, 236, 3, 28, 213, 28]]));
            let ticker = Ticker::new("BTC");
            PriceTimes::insert(ticker, 999999999999999);
            let msg = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000688e4cda00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000");
            let sig = hex_literal::hex!("69538bfa1a2097ea206780654d7baac3a17ee57547ee3eeb5d8bcb58a2fcdf401ff8834f4a003193f24224437881276fe76c8e1c0a361081de854457d41d0690000000000000000000000000000000000000000000000000000000000000001c");

            assert_eq!(
                validate_unsigned(
                    TransactionSource::External {},
                    &Call::post_prices::<Test>(vec![(msg.to_vec(), sig.to_vec())]),
                ),
                Err(ValidationError::InvalidPrice(OracleError::StalePrice))
            );
        });
    }

    #[test]
    fn test_post_prices_valid_remote() {
        new_test_ext().execute_with(|| {
            PriceReporters::put(ReporterSet(vec![[133, 97, 91, 7, 102, 21, 49, 124, 128, 241, 76, 186, 214, 80, 30, 236, 3, 28, 213, 28]]));

            let msg = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000688e4cda00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000");
            let sig = hex_literal::hex!("69538bfa1a2097ea206780654d7baac3a17ee57547ee3eeb5d8bcb58a2fcdf401ff8834f4a003193f24224437881276fe76c8e1c0a361081de854457d41d0690000000000000000000000000000000000000000000000000000000000000001c");

            assert_eq!(
                validate_unsigned(
                    TransactionSource::External {},
                    &Call::post_prices::<Test>(vec![(msg.to_vec(), sig.to_vec())]),
                ),
                Ok(ValidTransaction::with_tag_prefix("Gateway::post_prices")
                    .priority(UNSIGNED_TXS_PRIORITY)
                    .longevity(UNSIGNED_TXS_LONGEVITY)
                    .and_provides(vec![sig.to_vec()])
                    .propagate(true)
                    .build())
            );
        });
    }

    #[test]
    fn test_post_prices_valid_local() {
        new_test_ext().execute_with(|| {
            PriceReporters::put(ReporterSet(vec![[133, 97, 91, 7, 102, 21, 49, 124, 128, 241, 76, 186, 214, 80, 30, 236, 3, 28, 213, 28]]));

            let msg = hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005fec975800000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000688e4cda00000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034254430000000000000000000000000000000000000000000000000000000000");
            let sig = hex_literal::hex!("69538bfa1a2097ea206780654d7baac3a17ee57547ee3eeb5d8bcb58a2fcdf401ff8834f4a003193f24224437881276fe76c8e1c0a361081de854457d41d0690000000000000000000000000000000000000000000000000000000000000001c");

            assert_eq!(
                validate_unsigned(
                    TransactionSource::Local {},
                    &Call::post_prices::<Test>(vec![(msg.to_vec(), sig.to_vec())]),
                ),
                Ok(ValidTransaction::with_tag_prefix("Gateway::post_prices")
                    .priority(UNSIGNED_TXS_PRIORITY)
                    .longevity(UNSIGNED_TXS_LONGEVITY)
                    .and_provides(vec![sig.to_vec()])
                    .propagate(false)
                    .build())
            );
        });
    }
}
