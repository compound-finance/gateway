use crate::{
    chains::{
        Chain, ChainAccount, ChainHash, ChainId, ChainSignature, ChainSignatureList, Ethereum,
    },
    reason::Reason,
};
use codec::{Decode, Encode};
use ethabi::Token;
use our_std::{vec::Vec, RuntimeDebug};

use types_derive::{type_alias, Types};

/// Type for a generic encoded message, potentially for any chain.
#[type_alias]
pub type EncodedNotice = Vec<u8>;

#[type_alias]
pub type EraId = u32;

#[type_alias]
pub type EraIndex = u32;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug, Types)]
pub struct NoticeId(pub EraId, pub EraIndex);

impl NoticeId {
    pub fn seq(&self) -> NoticeId {
        NoticeId(self.0, self.1 + 1)
    }

    pub fn seq_era(&self) -> NoticeId {
        NoticeId(self.0 + 1, 0)
    }
}

impl NoticeId {
    pub fn era_id(&self) -> u32 {
        self.0
    }

    pub fn era_index(&self) -> u32 {
        self.1
    }
}

lazy_static! {
    static ref UNLOCK_SIG: <Ethereum as Chain>::Hash =
        <Ethereum as Chain>::hash_bytes(b"unlock(address,uint256,address)");
    static ref UNLOCK_CASH_SIG: <Ethereum as Chain>::Hash =
        <Ethereum as Chain>::hash_bytes(b"unlockCash(address,uint128)");
    static ref SET_FUTURE_YIELD_SIG: <Ethereum as Chain>::Hash =
        <Ethereum as Chain>::hash_bytes(b"setFutureYield(uint128,uint128,uint256)");
    static ref SET_SUPPLY_CAP_SIG: <Ethereum as Chain>::Hash =
        <Ethereum as Chain>::hash_bytes(b"setSupplyCap(address,uint256)");
    static ref CHANGE_AUTHORITIES_SIG: <Ethereum as Chain>::Hash =
        <Ethereum as Chain>::hash_bytes(b"changeAuthorities(address[])");
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ExtractionNotice {
    Eth {
        id: NoticeId,
        parent: <Ethereum as Chain>::Hash,
        asset: <Ethereum as Chain>::Address,
        account: <Ethereum as Chain>::Address,
        amount: <Ethereum as Chain>::Amount,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum CashExtractionNotice {
    Eth {
        id: NoticeId,
        parent: <Ethereum as Chain>::Hash,
        account: <Ethereum as Chain>::Address,
        principal: <Ethereum as Chain>::Amount,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum FutureYieldNotice {
    Eth {
        id: NoticeId,
        parent: <Ethereum as Chain>::Hash,
        next_cash_yield: <Ethereum as Chain>::Rate,
        next_cash_index: <Ethereum as Chain>::CashIndex,
        next_cash_yield_start: <Ethereum as Chain>::Timestamp,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum SetSupplyCapNotice {
    Eth {
        id: NoticeId,
        parent: <Ethereum as Chain>::Hash,
        asset: <Ethereum as Chain>::Address,
        cap: <Ethereum as Chain>::Amount,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum ChangeAuthorityNotice {
    Eth {
        id: NoticeId,
        parent: <Ethereum as Chain>::Hash,
        new_authorities: Vec<<Ethereum as Chain>::Address>,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum Notice {
    ExtractionNotice(ExtractionNotice),
    CashExtractionNotice(CashExtractionNotice),
    FutureYieldNotice(FutureYieldNotice),
    SetSupplyCapNotice(SetSupplyCapNotice),
    ChangeAuthorityNotice(ChangeAuthorityNotice),
}

impl Notice {
    pub fn hash(&self) -> ChainHash {
        self.chain_id().hash_bytes(&self.encode_notice()[..])
    }

    pub fn chain_id(&self) -> ChainId {
        match self {
            Notice::ExtractionNotice(n) => match n {
                ExtractionNotice::Eth { .. } => ChainId::Eth,
            },
            Notice::CashExtractionNotice(n) => match n {
                CashExtractionNotice::Eth { .. } => ChainId::Eth,
            },
            Notice::FutureYieldNotice(n) => match n {
                FutureYieldNotice::Eth { .. } => ChainId::Eth,
            },
            Notice::SetSupplyCapNotice(n) => match n {
                SetSupplyCapNotice::Eth { .. } => ChainId::Eth,
            },
            Notice::ChangeAuthorityNotice(n) => match n {
                ChangeAuthorityNotice::Eth { .. } => ChainId::Eth,
            },
        }
    }

    pub fn sign_notice(&self) -> Result<ChainSignature, Reason> {
        self.chain_id().sign(&self.encode_notice()[..])
    }
}

pub trait EncodeNotice {
    fn encode_notice(&self) -> EncodedNotice;
}

const ETH_CHAIN_IDENT: &'static [u8] = b"ETH:";

fn encode_notice_params(
    id: &NoticeId,
    parent: &<Ethereum as Chain>::Hash,
    signature: <Ethereum as Chain>::Hash,
    tokens: &[ethabi::Token],
) -> Vec<u8> {
    let mut result: Vec<u8> = ETH_CHAIN_IDENT.to_vec();
    let header_encoded = ethabi::encode(&[
        Token::Uint(id.era_id().into()),
        Token::Uint(id.era_index().into()),
        Token::Uint(parent.into()),
    ]);
    let abi_encoded = ethabi::encode(tokens);

    result.extend_from_slice(&header_encoded[..]);
    result.extend_from_slice(&signature[0..4]);
    result.extend_from_slice(&abi_encoded[..]);
    result
}

impl EncodeNotice for ExtractionNotice {
    fn encode_notice(&self) -> EncodedNotice {
        match self {
            ExtractionNotice::Eth {
                id,
                parent,
                asset,
                account,
                amount,
            } => encode_notice_params(
                id,
                parent,
                *UNLOCK_SIG,
                &[
                    Token::Address(asset.into()),
                    Token::Uint((*amount).into()),
                    Token::Address(account.into()),
                ],
            ),
        }
    }
}

impl EncodeNotice for CashExtractionNotice {
    fn encode_notice(&self) -> EncodedNotice {
        match self {
            CashExtractionNotice::Eth {
                id,
                parent,
                account,
                principal,
            } => encode_notice_params(
                id,
                parent,
                *UNLOCK_CASH_SIG,
                &[
                    Token::Address(account.into()),
                    Token::Uint((*principal).into()),
                ],
            ),
        }
    }
}

impl EncodeNotice for FutureYieldNotice {
    fn encode_notice(&self) -> EncodedNotice {
        match self {
            FutureYieldNotice::Eth {
                id,
                parent,
                next_cash_yield,
                next_cash_yield_start,
                next_cash_index,
            } => encode_notice_params(
                id,
                parent,
                *SET_FUTURE_YIELD_SIG,
                &[
                    Token::Uint((*next_cash_yield).into()),
                    Token::Uint((*next_cash_index).into()),
                    Token::Uint((*next_cash_yield_start).into()),
                ],
            ),
        }
    }
}

impl EncodeNotice for SetSupplyCapNotice {
    fn encode_notice(&self) -> EncodedNotice {
        match self {
            SetSupplyCapNotice::Eth {
                id,
                parent,
                asset,
                cap,
            } => encode_notice_params(
                id,
                parent,
                *SET_SUPPLY_CAP_SIG,
                &[Token::Address(asset.into()), Token::Uint((*cap).into())],
            ),
        }
    }
}

impl EncodeNotice for ChangeAuthorityNotice {
    fn encode_notice(&self) -> EncodedNotice {
        match self {
            ChangeAuthorityNotice::Eth {
                id,
                parent,
                new_authorities,
            } => encode_notice_params(
                id,
                parent,
                *CHANGE_AUTHORITIES_SIG,
                &[Token::Array(
                    new_authorities
                        .iter()
                        .map(|auth| Token::Address(auth.into()))
                        .collect(),
                )],
            ),
        }
    }
}

impl EncodeNotice for Notice {
    fn encode_notice(&self) -> EncodedNotice {
        match self {
            Notice::ExtractionNotice(n) => n.encode_notice(),
            Notice::CashExtractionNotice(n) => n.encode_notice(),
            Notice::FutureYieldNotice(n) => n.encode_notice(),
            Notice::SetSupplyCapNotice(n) => n.encode_notice(),
            Notice::ChangeAuthorityNotice(n) => n.encode_notice(),
        }
    }
}

pub fn default_notice_signatures(notice: &Notice) -> ChainSignatureList {
    match notice {
        Notice::ExtractionNotice(n) => match n {
            ExtractionNotice::Eth { .. } => ChainSignatureList::Eth(vec![]),
        },
        Notice::CashExtractionNotice(n) => match n {
            CashExtractionNotice::Eth { .. } => ChainSignatureList::Eth(vec![]),
        },
        Notice::FutureYieldNotice(n) => match n {
            FutureYieldNotice::Eth { .. } => ChainSignatureList::Eth(vec![]),
        },
        Notice::SetSupplyCapNotice(n) => match n {
            SetSupplyCapNotice::Eth { .. } => ChainSignatureList::Eth(vec![]),
        },
        Notice::ChangeAuthorityNotice(n) => match n {
            ChangeAuthorityNotice::Eth { .. } => ChainSignatureList::Eth(vec![]),
        },
    }
}

/// Type for the status of a notice on the queue.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, Types)]
pub enum NoticeState {
    Missing,
    Pending { signature_pairs: ChainSignatureList },
    Executed,
}

impl NoticeState {
    pub fn pending(notice: &Notice) -> Self {
        NoticeState::Pending {
            signature_pairs: default_notice_signatures(&notice),
        }
    }
}

impl Default for NoticeState {
    fn default() -> Self {
        NoticeState::Missing
    }
}

pub fn has_signer(signature_pairs: &ChainSignatureList, signer: ChainAccount) -> bool {
    match (signature_pairs, signer) {
        (ChainSignatureList::Eth(eth_signature_pairs), ChainAccount::Eth(eth_account)) => {
            eth_signature_pairs.iter().any(|(s, _)| s == &eth_account)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethabi::{Function, Param, ParamType, Token};

    #[test]
    fn test_encodes_extraction_notice() -> Result<(), ethabi::Error> {
        let asset = [2u8; 20];
        let amount = 50;
        let account = [1u8; 20];

        let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
            id: NoticeId(80, 1),
            parent: [3u8; 32],
            asset,
            amount,
            account,
        });

        let expected = [
            69, 84, 72, 58, // ETH:
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 80, // eraId
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, // eraIndex
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, // parent
            139, 195, 146, 7, // Function Signature (0x8bc39207)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, // asset
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 50, // amount
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, // account
        ];
        let encoded = notice.encode_notice();
        assert_eq!(encoded, expected);

        // Test against auto-encoding
        let asset_token = Token::Address(asset.into());
        let amount_token = Token::Uint(amount.into());
        let account_token = Token::Address(account.into());

        let unlock_fn = Function {
            name: String::from("unlock"),
            inputs: vec![
                Param {
                    name: String::from("asset"),
                    kind: ParamType::Address,
                },
                Param {
                    name: String::from("amount"),
                    kind: ParamType::Uint(256),
                },
                Param {
                    name: String::from("account"),
                    kind: ParamType::Address,
                },
            ],
            outputs: vec![],
            constant: false,
        };
        assert_eq!(
            &unlock_fn.encode_input(&[asset_token, amount_token, account_token])?[..],
            &expected[100..]
        );
        Ok(())
    }

    #[test]
    fn test_encodes_cash_extraction_notice() -> Result<(), ethabi::Error> {
        let account = [1u8; 20];
        let principal = 50;

        let notice = Notice::CashExtractionNotice(CashExtractionNotice::Eth {
            id: NoticeId(80, 1),
            parent: [3u8; 32],
            account,
            principal,
        });

        let expected = [
            69, 84, 72, 58, // ETH:
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 80, // eraId
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, // eraIndex
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, // parent
            0x4c, 0xfa, 0x3d, 0xa3, // Function Signature (0x4cfa3da3)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, // account
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 50, // principal
        ];
        let encoded = notice.encode_notice();
        assert_eq!(encoded, expected);

        // Test against auto-encoding
        let account_token = Token::Address(account.into());
        let principal_token = Token::Uint(principal.into());

        let unlock_cash_fn = Function {
            name: String::from("unlockCash"),
            inputs: vec![
                Param {
                    name: String::from("account"),
                    kind: ParamType::Address,
                },
                Param {
                    name: String::from("principal"),
                    kind: ParamType::Uint(128),
                },
            ],
            outputs: vec![],
            constant: false,
        };
        assert_eq!(
            &unlock_cash_fn.encode_input(&[account_token, principal_token])?[..],
            &expected[100..]
        );
        Ok(())
    }

    #[test]
    fn test_encodes_future_yield_notice() -> Result<(), ethabi::Error> {
        let next_cash_yield = 700u128;
        let next_cash_yield_start = 200u64;
        let next_cash_index = 400u128;

        let notice = Notice::FutureYieldNotice(FutureYieldNotice::Eth {
            id: NoticeId(80, 1),
            parent: [3u8; 32],
            next_cash_yield,
            next_cash_index,
            next_cash_yield_start,
        });

        let expected = [
            69, 84, 72, 58, // ETH:
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 80, // eraId
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, // eraIndex
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, // parent
            0x1e, 0x9d, 0x77, 0xd9, // Function Signature (0x1e9d77d9)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0x02, 0xbc, // next_cash_yield
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0x01, 0x90, // next_cash_index
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 200, // next_cash_yield_start
        ];
        let encoded = notice.encode_notice();
        assert_eq!(encoded, expected);

        // Test against auto-encoding
        let next_cash_yield_token = Token::Uint(next_cash_yield.into());
        let next_cash_yield_start_at_token = Token::Uint(next_cash_yield_start.into());
        let next_cash_index_token = Token::Uint(next_cash_index.into());

        let set_future_yield_fn = Function {
            name: String::from("setFutureYield"),
            inputs: vec![
                Param {
                    name: String::from("nextCashYield"),
                    kind: ParamType::Uint(128),
                },
                Param {
                    name: String::from("nextCashYieldIndex"),
                    kind: ParamType::Uint(128),
                },
                Param {
                    name: String::from("nextCashYieldStart"),
                    kind: ParamType::Uint(256),
                },
            ],
            outputs: vec![],
            constant: false,
        };
        assert_eq!(
            &set_future_yield_fn.encode_input(&[
                next_cash_yield_token,
                next_cash_index_token,
                next_cash_yield_start_at_token,
            ])?[..],
            &expected[100..]
        );
        Ok(())
    }

    #[test]
    fn test_encodes_set_supply_cap_notice() -> Result<(), ethabi::Error> {
        let asset = [2u8; 20];
        let cap = 50;

        let notice = Notice::SetSupplyCapNotice(SetSupplyCapNotice::Eth {
            id: NoticeId(80, 1),
            parent: [3u8; 32],
            asset,
            cap,
        });

        let expected = [
            69, 84, 72, 58, // ETH:
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 80, // eraId
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, // eraIndex
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, // parent
            0x57, 0x1f, 0x03, 0xe5, // Function Signature (0x571f03e5)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, // asset
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 50, // cap
        ];
        let encoded = notice.encode_notice();
        assert_eq!(encoded, expected);

        // Test against auto-encoding
        let asset_token = Token::Address(asset.into());
        let supply_cap_token = Token::Uint(cap.into());

        let set_supply_cap_fn = Function {
            name: String::from("setSupplyCap"),
            inputs: vec![
                Param {
                    name: String::from("asset"),
                    kind: ParamType::Address,
                },
                Param {
                    name: String::from("supplyCap"),
                    kind: ParamType::Uint(256),
                },
            ],
            outputs: vec![],
            constant: false,
        };
        assert_eq!(
            &set_supply_cap_fn.encode_input(&[asset_token, supply_cap_token])?[..],
            &expected[100..]
        );
        Ok(())
    }

    #[test]
    fn test_encodes_change_authorities_notice() -> Result<(), ethabi::Error> {
        let new_authorities = vec![[6u8; 20], [7u8; 20], [8u8; 20]];

        let notice = Notice::ChangeAuthorityNotice(ChangeAuthorityNotice::Eth {
            id: NoticeId(80, 1),
            parent: [3u8; 32],
            new_authorities: new_authorities.clone(),
        });

        let expected = [
            69, 84, 72, 58, // ETH:
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 80, // eraId
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, // eraIndex
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, // parent
            0x14, 0xee, 0x45, 0xf2, // Function Signature (0x14ee45f2)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0x20, // data offset
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 3, // authorities count
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
            6, 6, 6, // vec[0]
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, // vec[1]
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
            8, 8, 8, // vec[2]
        ];
        let encoded = notice.encode_notice();
        assert_eq!(encoded, expected);

        // Test against auto-encoding
        let new_authorities_token = Token::Array(
            new_authorities
                .iter()
                .map(|auth| Token::Address(auth.into()))
                .collect(),
        );

        let change_authorities_fn = Function {
            name: String::from("changeAuthorities"),
            inputs: vec![Param {
                name: String::from("newAuthorities"),
                kind: ParamType::Array(Box::new(ParamType::Address)),
            }],
            outputs: vec![],
            constant: false,
        };
        assert_eq!(
            &change_authorities_fn.encode_input(&[new_authorities_token])?[..],
            &expected[100..]
        );
        Ok(())
    }
}
