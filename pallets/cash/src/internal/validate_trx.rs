use crate::{
    chains::{Chain, ChainAccount, Ethereum},
    internal,
    notices::EncodeNotice,
    reason::Reason,
    AllowedNextCodeHash, Call, Config, Notices, Validators,
};
use codec::Encode;
use frame_support::storage::{IterableStorageMap, StorageDoubleMap, StorageValue};
use our_std::RuntimeDebug;
use sp_runtime::transaction_validity::{TransactionSource, TransactionValidity, ValidTransaction};

#[derive(Eq, PartialEq, RuntimeDebug)]
pub enum ValidationError {
    InvalidInternalOnly,
    InvalidNextCode,
    InvalidValidator,
    InvalidSignature,
    InvalidCall,
    InvalidPriceSignature,
    InvalidPrice(Reason),
    UnknownNotice,
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
                        .priority(100)
                        .longevity(32)
                        .and_provides(hash)
                        .propagate(true)
                        .build(),
                )
            } else {
                Err(ValidationError::InvalidNextCode)
            }
        }
        Call::receive_event(event_id, event, signature) => {
            if let Ok(signer) = runtime_interfaces::keyring_interface::eth_recover(
                event.encode(),
                signature.clone(),
                false,
            ) {
                let validators: Vec<_> = Validators::iter().map(|v| v.1.eth_address).collect();
                if validators.contains(&signer) {
                    Ok(ValidTransaction::with_tag_prefix("Gateway::receive_event")
                        .priority(100)
                        .longevity(32)
                        .and_provides((event_id, signature))
                        .propagate(true)
                        .build())
                } else {
                    Err(ValidationError::InvalidValidator)
                }
            } else {
                Err(ValidationError::InvalidSignature)
            }
        }
        Call::exec_trx_request(request, signature, nonce) => {
            let signer_res = signature
                .recover_account(&internal::exec_trx_request::prepend_nonce(request, *nonce)[..]);

            // TODO: We might want to check existential value of signer

            match (signer_res, nonce) {
                (Err(_e), _) => Err(ValidationError::InvalidSignature),
                (Ok(sender), nonce) => Ok(ValidTransaction::with_tag_prefix(
                    "Gateway::exec_trx_request",
                )
                .priority(100)
                .longevity(32)
                .and_provides((sender, nonce))
                .propagate(true)
                .build()),
            }
        }
        Call::publish_signature(chain_id, notice_id, signature) => {
            let notice = Notices::get(chain_id, notice_id).ok_or(ValidationError::UnknownNotice)?;
            let signer = signature
                .recover(&notice.encode_notice())
                .map_err(|_| ValidationError::InvalidSignature)?;

            if Validators::iter().any(|(_, v)| ChainAccount::Eth(v.eth_address) == signer) {
                Ok(
                    ValidTransaction::with_tag_prefix("Gateway::publish_signature")
                        .priority(100)
                        .longevity(32)
                        .and_provides(signature)
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
    use crate::{
        chains::{
            Chain, ChainAccount, ChainAccountSignature, ChainId, ChainSignature,
            ChainSignatureList, Ethereum,
        },
        events::{ChainLogEvent, ChainLogId},
        mock::*,
        notices::{ExtractionNotice, Notice, NoticeId, NoticeState},
        types::{ValidatorKeys, ValidatorSig},
        Call, NoticeStates, Validators,
    };
    use ethereum_client::{events::EthereumEvent::Lock, EthereumLogEvent};
    use frame_support::storage::StorageMap;

    use sp_core::crypto::AccountId32;

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
    fn test_receive_event_recover_failure() {
        new_test_ext().execute_with(|| {
            let event_id: ChainLogId = ChainLogId::Eth(1, 1);
            let event: ChainLogEvent = ChainLogEvent::Eth(EthereumLogEvent {
                block_hash: [0u8; 32],
                block_number: 1,
                transaction_index: 1,
                log_index: 1,
                event: Lock {
                    asset: [0u8; 20],
                    sender: [0u8; 20],
                    chain: String::from("ETH"),
                    recipient: [0u8; 32],
                    amount: 500,
                },
            });
            let signature: ValidatorSig = [0u8; 65];

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::receive_event::<Test>(event_id, event, signature)
                ),
                Err(ValidationError::InvalidSignature)
            );
        });
    }

    #[test]
    fn test_receive_event_not_a_validator() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();

            let event_id: ChainLogId = ChainLogId::Eth(1, 1);
            let event: ChainLogEvent = ChainLogEvent::Eth(EthereumLogEvent {
                block_hash: [0u8; 32],
                block_number: 1,
                transaction_index: 1,
                log_index: 1,
                event: Lock {
                    asset: [0u8; 20],
                    sender: [0u8; 20],
                    chain: String::from("ETH"),
                    recipient: [0u8; 32],
                    amount: 500,
                },
            });
            let eth_signature = match event.sign_event().unwrap() {
                ChainSignature::Eth(s) => s,
                _ => panic!("absurd"),
            };

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::receive_event::<Test>(event_id, event, eth_signature)
                ),
                Err(ValidationError::InvalidValidator)
            );
        });
    }

    #[test]
    fn test_receive_event_is_validator() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
            let substrate_id = AccountId32::new([0u8; 32]);
            let eth_address = <Ethereum as Chain>::signer_address().unwrap();
            Validators::insert(
                substrate_id.clone(),
                ValidatorKeys {
                    substrate_id,
                    eth_address,
                },
            );

            let event_id: ChainLogId = ChainLogId::Eth(1, 1);
            let event: ChainLogEvent = ChainLogEvent::Eth(EthereumLogEvent {
                block_hash: [0u8; 32],
                block_number: 1,
                transaction_index: 1,
                log_index: 1,
                event: Lock {
                    asset: [0u8; 20],
                    sender: [0u8; 20],
                    chain: String::from("ETH"),
                    recipient: [0u8; 32],
                    amount: 500,
                },
            });
            let eth_signature = match event.sign_event().unwrap() {
                ChainSignature::Eth(s) => s,
                _ => panic!("absurd"),
            };
            let exp = ValidTransaction::with_tag_prefix("Gateway::receive_event")
                .priority(100)
                .longevity(32)
                .and_provides((event_id, eth_signature))
                .propagate(true)
                .build();

            assert_eq!(
                validate_unsigned(
                    TransactionSource::InBlock {},
                    &Call::receive_event::<Test>(event_id, event, eth_signature)
                ),
                Ok(exp)
            );
        });
    }

    #[test]
    fn test_exec_trx_request_nonce_zero() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
            let request: Vec<u8> = String::from("Hello").as_bytes().into();
            let nonce = 0;
            let full_request: Vec<u8> = format!("\x19Ethereum Signed Message:\n70:Hello")
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
            runtime_interfaces::set_validator_config_dev_defaults();
            let request: Vec<u8> = String::from("Hello").as_bytes().into();
            let nonce = 5;
            let full_request: Vec<u8> = format!("\x19Ethereum Signed Message:\n75:Hello")
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
                .and_provides((ChainAccount::Eth(eth_address), 5))
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
    fn test_publish_signature_invalid_signature() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
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
                Err(ValidationError::InvalidSignature)
            );
        });
    }

    #[test]
    fn test_publish_signature_invalid_validator() {
        new_test_ext().execute_with(|| {
            runtime_interfaces::set_validator_config_dev_defaults();
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
            runtime_interfaces::set_validator_config_dev_defaults();
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
            let substrate_id = AccountId32::new([0u8; 32]);
            Validators::insert(
                substrate_id.clone(),
                ValidatorKeys {
                    substrate_id,
                    eth_address: signer,
                },
            );

            let exp = ValidTransaction::with_tag_prefix("Gateway::publish_signature")
                .priority(100)
                .longevity(32)
                .and_provides(signature)
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
