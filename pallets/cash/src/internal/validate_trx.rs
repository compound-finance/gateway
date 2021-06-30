use crate::{
    chains::{Chain, Ethereum},
    core::recover_validator,
    internal,
    notices::EncodeNotice,
    params::{UNSIGNED_TXS_LONGEVITY, UNSIGNED_TXS_PRIORITY},
    reason::Reason,
    AllowedNextCodeHash, Call, Config, Notices, Validators,
};
use codec::Encode;
use frame_support::storage::{IterableStorageMap, StorageDoubleMap, StorageValue};
use our_std::{log, RuntimeDebug};
use sp_runtime::transaction_validity::{TransactionSource, TransactionValidity, ValidTransaction};

#[derive(Eq, PartialEq, RuntimeDebug, Clone, Copy)]
pub enum ValidationError {
    InvalidInternalOnly,
    InvalidNextCode,
    InvalidValidator,
    InvalidCall,
    InvalidPriceSignature,
    InvalidPrice(Reason),
    UnknownNotice,
    InvalidTrxRequest(Reason),
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
        Call::set_miner(_miner) => match source {
            TransactionSource::InBlock => {
                Ok(ValidTransaction::with_tag_prefix("Gateway::set_miner")
                    .longevity(1)
                    .build())
            }
            _ => Err(ValidationError::InvalidInternalOnly),
        },

        Call::set_next_code_via_hash(next_code) => {
            let hash = <Ethereum as Chain>::hash_bytes(&next_code);

            if AllowedNextCodeHash::get() == Some(hash) {
                Ok(
                    ValidTransaction::with_tag_prefix("Gateway::set_next_code_via_hash")
                        .priority(UNSIGNED_TXS_PRIORITY)
                        .longevity(UNSIGNED_TXS_LONGEVITY)
                        .and_provides(hash)
                        .propagate(true)
                        .build(),
                )
            } else {
                Err(ValidationError::InvalidNextCode)
            }
        }

        Call::set_starport(starport) => {
            Ok(ValidTransaction::with_tag_prefix("Gateway::set_starport")
                .priority(UNSIGNED_TXS_PRIORITY)
                .longevity(UNSIGNED_TXS_LONGEVITY)
                .and_provides(starport)
                .propagate(true)
                .build())
        }

        Call::set_genesis_block(genesis_block) => Ok(ValidTransaction::with_tag_prefix(
            "Gateway::set_genesis_block",
        )
        .priority(UNSIGNED_TXS_PRIORITY)
        .longevity(UNSIGNED_TXS_LONGEVITY)
        .and_provides(genesis_block)
        .propagate(true)
        .build()),

        Call::receive_chain_blocks(blocks, signature) => {
            let chain_id = blocks.chain_id();

            let validator = recover_validator::<T>(&blocks.encode(), *signature)
                .map_err(|_| ValidationError::InvalidValidator)?;

            let mut validity = ValidTransaction::with_tag_prefix("Gateway::receive_chain_blocks")
                .priority(UNSIGNED_TXS_PRIORITY)
                .longevity(UNSIGNED_TXS_LONGEVITY)
                .propagate(true);

            for block_number in blocks.block_numbers() {
                validity =
                    validity.and_provides((validator.substrate_id.clone(), block_number, chain_id));
            }

            Ok(validity.build())
        }

        Call::receive_chain_reorg(reorg, signature) => {
            let _validator = recover_validator::<T>(&reorg.encode(), *signature)
                .map_err(|_| ValidationError::InvalidValidator)?;
            Ok(
                ValidTransaction::with_tag_prefix("Gateway::receive_chain_reorg")
                    .priority(100)
                    .longevity(32)
                    .and_provides(reorg)
                    .propagate(true)
                    .build(),
            )
        }

        Call::exec_trx_request(request, signature, nonce) => {
            let signer_res = internal::exec_trx_request::is_minimally_valid_trx_request::<T>(
                request.to_vec(),
                *signature,
                *nonce,
            );

            match (signer_res, nonce) {
                (Err(e), _) => Err(ValidationError::InvalidTrxRequest(e)),
                (Ok((sender, current_nonce)), nonce) => {
                    // Nonce check
                    if current_nonce == 0 || *nonce == current_nonce {
                        Ok(
                            ValidTransaction::with_tag_prefix("Gateway::exec_trx_request")
                                .priority(UNSIGNED_TXS_PRIORITY)
                                .longevity(UNSIGNED_TXS_LONGEVITY)
                                .and_provides((sender, nonce))
                                .and_provides(request)
                                .propagate(true)
                                .build(),
                        )
                    } else {
                        Ok(
                            ValidTransaction::with_tag_prefix("Gateway::exec_trx_request")
                                .priority(UNSIGNED_TXS_PRIORITY)
                                .longevity(UNSIGNED_TXS_LONGEVITY)
                                .and_requires((sender, nonce - 1))
                                .and_provides((sender, nonce))
                                .and_provides(request)
                                .propagate(true)
                                .build(),
                        )
                    }
                }
            }
        }

        Call::publish_signature(chain_id, notice_id, signature) => {
            let notice = Notices::get(chain_id, notice_id).ok_or(ValidationError::UnknownNotice)?;
            let validator = recover_validator::<T>(&notice.encode_notice(), *signature)
                .map_err(|_| ValidationError::InvalidValidator)?;

            // XXX what happens if not eth here? seems broken
            if Validators::iter().any(|(_, v)| v.eth_address == validator.eth_address) {
                Ok(
                    ValidTransaction::with_tag_prefix("Gateway::publish_signature")
                        .priority(UNSIGNED_TXS_PRIORITY)
                        .longevity(UNSIGNED_TXS_LONGEVITY)
                        .and_provides((chain_id, notice_id, signature))
                        .propagate(true)
                        .build(),
                )
            } else {
                Err(ValidationError::InvalidValidator)
            }
        }
        _ => Err(ValidationError::InvalidCall),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::validator_sign, tests::*, Call};
    use ethereum_client::EthereumBlock;

    #[test]
    fn test_set_miner_external() {
        new_test_ext().execute_with(|| {
            let miner = ChainAccount::Eth([0u8; 20]);
            assert_eq!(
                validate_unsigned(
                    TransactionSource::External {},
                    &Call::set_miner::<Test>(miner),
                ),
                Err(ValidationError::InvalidInternalOnly)
            );
        });
    }

    #[test]
    fn test_set_miner_in_block() {
        new_test_ext().execute_with(|| {
            let miner = ChainAccount::Eth([0u8; 20]);
            let exp = ValidTransaction::with_tag_prefix("Gateway::set_miner")
                .longevity(1)
                .build();

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::set_miner::<Test>(miner),
                ),
                Ok(exp)
            );
        });
    }

    #[test]
    fn test_set_next_code_via_hash_not_exists() {
        new_test_ext().execute_with(|| {
            let next_code: Vec<u8> = [0u8; 10].into();

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::set_next_code_via_hash::<Test>(next_code),
                ),
                Err(ValidationError::InvalidNextCode)
            );
        });
    }

    #[test]
    fn test_set_next_code_via_hash_exists_mismatch() {
        new_test_ext().execute_with(|| {
            AllowedNextCodeHash::put([0u8; 32]);
            let next_code: Vec<u8> = [0u8; 10].into();

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::set_next_code_via_hash::<Test>(next_code),
                ),
                Err(ValidationError::InvalidNextCode)
            );
        });
    }

    #[test]
    fn test_set_next_code_via_hash_exists_match() {
        new_test_ext().execute_with(|| {
            let next_code: Vec<u8> = [0u8; 10].into();
            let hash = <Ethereum as Chain>::hash_bytes(&next_code);
            AllowedNextCodeHash::put(hash);
            let exp = ValidTransaction::with_tag_prefix("Gateway::set_next_code_via_hash")
                .priority(100)
                .longevity(32)
                .and_provides(hash)
                .propagate(true)
                .build();

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::set_next_code_via_hash::<Test>(next_code),
                ),
                Ok(exp)
            );
        });
    }

    #[test]
    fn test_receive_chain_blocks_recover_failure() {
        new_test_ext().execute_with(|| {
            let blocks = ChainBlocks::Eth(vec![]);
            let signature = ChainSignature::Eth([0u8; 65]);
            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::receive_chain_blocks::<Test>(blocks, signature)
                ),
                Err(ValidationError::InvalidValidator)
            );
        });
    }

    #[test]
    fn test_receive_chain_blocks_not_a_validator() {
        new_test_ext().execute_with(|| {
            let blocks = ChainBlocks::Eth(vec![]);
            let signature = validator_sign::<Test>(&blocks.encode()).unwrap();

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::receive_chain_blocks::<Test>(blocks, signature)
                ),
                Err(ValidationError::InvalidValidator)
            );
        });
    }

    #[test]
    fn test_receive_chain_blocks_is_validator() {
        new_test_ext().execute_with(|| {
            let substrate_id = AccountId32::new([1u8; 32]);
            let eth_address = <Ethereum as Chain>::signer_address().unwrap();
            Validators::insert(
                substrate_id.clone(),
                ValidatorKeys {
                    substrate_id: substrate_id.clone(),
                    eth_address,
                },
            );

            let blocks = ChainBlocks::Eth(vec![
                EthereumBlock {
                    hash: [1; 32],
                    parent_hash: [0; 32],
                    number: 1,
                    events: vec![],
                },
                EthereumBlock {
                    hash: [2; 32],
                    parent_hash: [1; 32],
                    number: 2,
                    events: vec![],
                },
                EthereumBlock {
                    hash: [3; 32],
                    parent_hash: [2; 32],
                    number: 3,
                    events: vec![],
                },
            ]);
            let signature = validator_sign::<Test>(&blocks.encode()).unwrap();
            let exp = ValidTransaction::with_tag_prefix("Gateway::receive_chain_blocks")
                .priority(100)
                .longevity(32)
                .propagate(true)
                .and_provides((substrate_id.clone(), 1u64, ChainId::Eth))
                .and_provides((substrate_id.clone(), 2u64, ChainId::Eth))
                .and_provides((substrate_id.clone(), 3u64, ChainId::Eth))
                .build();

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::receive_chain_blocks::<Test>(blocks, signature)
                ),
                Ok(exp)
            );
        });
    }

    #[test]
    fn test_exec_trx_request_nonce_zero() {
        new_test_ext().execute_with(|| {
            let request: Vec<u8> = String::from("(Extract 50000000 Cash Eth:0xfc04833Ca66b7D6B4F540d4C2544228f64a25ac2)").as_bytes().into();
            let nonce = 0;
            let full_request: Vec<u8> = format!("\x19Ethereum Signed Message:\n720:(Extract 50000000 Cash Eth:0xfc04833Ca66b7D6B4F540d4C2544228f64a25ac2)")
                .as_bytes()
                .into();
            let eth_address = <Ethereum as Chain>::signer_address().unwrap();
            let eth_key_id =
                runtime_interfaces::validator_config_interface::get_eth_key_id().unwrap();
            let signature_raw =
                runtime_interfaces::keyring_interface::sign_one(full_request, eth_key_id).unwrap();

            let signature = ChainAccountSignature::Eth(eth_address, signature_raw);

            let exp = ValidTransaction::with_tag_prefix("Gateway::exec_trx_request")
                .priority(100)
                .longevity(32)
                .and_provides((ChainAccount::Eth(eth_address), 0))
                .and_provides(request.clone())
                .propagate(true)
                .build();

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::exec_trx_request::<Test>(request, signature, nonce),
                ),
                Ok(exp)
            );
        });
    }

    #[test]
    fn test_exec_trx_request_nonce_nonzero() {
        new_test_ext().execute_with(|| {
            let request: Vec<u8> = String::from(
                "(Extract 50000000 Cash Eth:0xfc04833Ca66b7D6B4F540d4C2544228f64a25ac2)",
            )
            .as_bytes()
            .into();
            let nonce = 5;
            let full_request: Vec<u8> = format!("\x19Ethereum Signed Message:\n725:(Extract 50000000 Cash Eth:0xfc04833Ca66b7D6B4F540d4C2544228f64a25ac2)")
                .as_bytes()
                .into();
            let eth_address = <Ethereum as Chain>::signer_address().unwrap();
            let eth_key_id =
                runtime_interfaces::validator_config_interface::get_eth_key_id().unwrap();
            let signature_raw =
                runtime_interfaces::keyring_interface::sign_one(full_request, eth_key_id).unwrap();

            let signature = ChainAccountSignature::Eth(eth_address, signature_raw);

            Nonces::insert(ChainAccount::Eth(eth_address), nonce);

            let exp = ValidTransaction::with_tag_prefix("Gateway::exec_trx_request")
                .priority(UNSIGNED_TXS_PRIORITY)
                .longevity(UNSIGNED_TXS_LONGEVITY)
                .and_provides((ChainAccount::Eth(eth_address), 5))
                .and_provides(request.clone())
                .propagate(true)
                .build();

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::exec_trx_request::<Test>(request, signature, nonce),
                ),
                Ok(exp)
            );
        });
    }

    #[test]
    fn test_exec_trx_request_valid_request_wrong_nonce() {
        new_test_ext().execute_with(|| {
            let request: Vec<u8> = String::from(
                "(Extract 50000000 Cash Eth:0xfc04833Ca66b7D6B4F540d4C2544228f64a25ac2)",
            )
            .as_bytes()
            .into();
            let nonce = 5;
            let full_request: Vec<u8> = format!("\x19Ethereum Signed Message:\n725:(Extract 50000000 Cash Eth:0xfc04833Ca66b7D6B4F540d4C2544228f64a25ac2)")
                .as_bytes()
                .into();
            let eth_address = <Ethereum as Chain>::signer_address().unwrap();
            let eth_key_id =
                runtime_interfaces::validator_config_interface::get_eth_key_id().unwrap();
            let signature_raw =
                runtime_interfaces::keyring_interface::sign_one(full_request, eth_key_id).unwrap();
            let signature = ChainAccountSignature::Eth(eth_address, signature_raw);

            Nonces::insert(ChainAccount::Eth(eth_address), nonce - 1);

            let exp = ValidTransaction::with_tag_prefix("Gateway::exec_trx_request")
                .priority(UNSIGNED_TXS_PRIORITY)
                .longevity(UNSIGNED_TXS_LONGEVITY)
                .and_requires((ChainAccount::Eth(eth_address), nonce - 1))
                .and_provides((ChainAccount::Eth(eth_address), nonce))
                .and_provides(request.clone())
                .propagate(true)
                .build();


            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::exec_trx_request::<Test>(request, signature, nonce),
                ),
                Ok(exp)
            );
        });
    }

    #[test]
    fn test_exec_trx_request_invalid_request_parse_error() {
        new_test_ext().execute_with(|| {
            let request: Vec<u8> = String::from("Parse Error").as_bytes().into();
            let nonce = 5;
            let full_request: Vec<u8> = format!("\x19Ethereum Signed Message:\n135:Parse Error")
                .as_bytes()
                .into();
            let eth_address = <Ethereum as Chain>::signer_address().unwrap();
            let eth_key_id =
                runtime_interfaces::validator_config_interface::get_eth_key_id().unwrap();
            let signature_raw =
                runtime_interfaces::keyring_interface::sign_one(full_request, eth_key_id).unwrap();
            let signature = ChainAccountSignature::Eth(eth_address, signature_raw);

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::exec_trx_request::<Test>(request, signature, nonce),
                ),
                Err(ValidationError::InvalidTrxRequest(
                    Reason::TrxRequestParseError(TrxReqParseError::InvalidExpression)
                ))
            );
        });
    }

    #[test]
    fn test_exec_trx_request_invalid_request_invalid_signature() {
        new_test_ext().execute_with(|| {
            let request: Vec<u8> = String::from(
                "(Extract 50000000 Cash Eth:0xfc04833Ca66b7D6B4F540d4C2544228f64a25ac2)",
            )
            .as_bytes()
            .into();
            let nonce = 5;
            let full_request: Vec<u8> = format!("\x19Ethereum Signed Message:\n45:(Extract 50000000 Cash Eth:0xfc04833Ca66b7D6B4F540d4C2544228f64a25ac2)")
                .as_bytes()
                .into();
            let eth_address = <Ethereum as Chain>::signer_address().unwrap();
            let eth_key_id =
                runtime_interfaces::validator_config_interface::get_eth_key_id().unwrap();
            let signature_raw =
                runtime_interfaces::keyring_interface::sign_one(full_request, eth_key_id).unwrap();
            let signature = ChainAccountSignature::Eth(eth_address, signature_raw);

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::exec_trx_request::<Test>(request, signature, nonce),
                ),
                Err(ValidationError::InvalidTrxRequest(
                    Reason::SignatureAccountMismatch
                ))
            );
        });
    }

    #[test]
    fn test_publish_signature_invalid_signature() {
        new_test_ext().execute_with(|| {
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });
            let mut signature = notice.sign_notice().unwrap();
            let eth_signature = match signature {
                ChainSignature::Eth(ref mut a) => {
                    a[64] = 2;
                    a
                }
                _ => panic!("invalid signature"),
            };
            let signer = <Ethereum as Chain>::signer_address().unwrap();
            let notice_state = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Eth(vec![(signer, *eth_signature)]),
            };
            NoticeStates::insert(chain_id, notice_id, notice_state);
            Notices::insert(chain_id, notice_id, notice);

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::publish_signature::<Test>(chain_id, notice_id, signature),
                ),
                Err(ValidationError::InvalidValidator)
            );
        });
    }

    #[test]
    fn test_publish_signature_invalid_validator() {
        new_test_ext().execute_with(|| {
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });
            let signature = notice.sign_notice().unwrap();
            let eth_signature = match signature {
                ChainSignature::Eth(a) => a,
                _ => panic!("invalid signature"),
            };
            let signer = <Ethereum as Chain>::signer_address().unwrap();
            let notice_state = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Eth(vec![(signer, eth_signature)]),
            };
            NoticeStates::insert(chain_id, notice_id, notice_state);
            Notices::insert(chain_id, notice_id, notice);

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::publish_signature::<Test>(chain_id, notice_id, signature),
                ),
                Err(ValidationError::InvalidValidator)
            );
        });
    }

    #[test]
    fn test_publish_signature_valid() {
        new_test_ext().execute_with(|| {
            let chain_id = ChainId::Eth;
            let notice_id = NoticeId(5, 6);
            let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
                id: NoticeId(80, 1),
                parent: [3u8; 32],
                asset: [1; 20],
                amount: 100,
                account: [2; 20],
            });
            let signer = <Ethereum as Chain>::signer_address().unwrap();
            let signature = notice.sign_notice().unwrap();
            let eth_signature = match signature {
                ChainSignature::Eth(a) => a,
                _ => panic!("invalid signature"),
            };
            let notice_state = NoticeState::Pending {
                signature_pairs: ChainSignatureList::Eth(vec![(signer, eth_signature)]),
            };
            NoticeStates::insert(chain_id, notice_id, notice_state);
            Notices::insert(chain_id, notice_id, notice);
            let substrate_id = AccountId32::new([0u8; 32]);
            Validators::insert(
                substrate_id.clone(),
                ValidatorKeys {
                    substrate_id,
                    eth_address: signer,
                },
            );

            let exp = ValidTransaction::with_tag_prefix("Gateway::publish_signature")
                .priority(UNSIGNED_TXS_PRIORITY)
                .longevity(UNSIGNED_TXS_LONGEVITY)
                .and_provides((chain_id, notice_id, signature))
                .propagate(true)
                .build();

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::publish_signature::<Test>(chain_id, notice_id, signature),
                ),
                Ok(exp)
            );
        });
    }

    #[test]
    fn test_other() {
        new_test_ext().execute_with(|| {
            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::change_validators::<Test>(vec![]),
                ),
                Err(ValidationError::InvalidCall)
            );
        });
    }
}
