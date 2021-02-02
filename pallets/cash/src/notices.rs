use crate::{
    chains::{Chain, ChainAccount, ChainSignature, ChainSignatureList, Ethereum},
    reason::Reason,
};
use codec::{Decode, Encode};
use our_std::{vec::Vec, RuntimeDebug};

/// Type for a generic encoded message, potentially for any chain.
pub type EncodedNotice = Vec<u8>;

pub type EraId = u32;
pub type EraIndex = u32;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct NoticeId(pub EraId, pub EraIndex);

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ExtractionNotice {
    Eth {
        id: NoticeId,
        parent: <Ethereum as Chain>::Hash,
        asset: <Ethereum as Chain>::Address,
        account: <Ethereum as Chain>::Address,
        amount: <Ethereum as Chain>::Amount,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum CashExtractionNotice {
    Eth {
        id: NoticeId,
        parent: <Ethereum as Chain>::Hash,
        account: <Ethereum as Chain>::Address,
        amount: <Ethereum as Chain>::Amount,
        cash_index: <Ethereum as Chain>::CashIndex,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum FutureYieldNotice {
    Eth {
        id: NoticeId,
        parent: <Ethereum as Chain>::Hash,
        next_cash_yield: <Ethereum as Chain>::Rate,
        next_cash_yield_start_at: <Ethereum as Chain>::Timestamp,
        next_cash_index: <Ethereum as Chain>::CashIndex,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum SetSupplyCapNotice {
    Eth {
        id: NoticeId,
        parent: <Ethereum as Chain>::Hash,
        asset: <Ethereum as Chain>::Address,
        amount: <Ethereum as Chain>::Amount,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum ChangeAuthorityNotice {
    Eth {
        id: NoticeId,
        parent: <Ethereum as Chain>::Hash,
        new_authorities: Vec<<Ethereum as Chain>::Address>,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum Notice {
    ExtractionNotice(ExtractionNotice),
    CashExtractionNotice(CashExtractionNotice),
    FutureYieldNotice(FutureYieldNotice),
    SetSupplyCapNotice(SetSupplyCapNotice),
    ChangeAuthorityNotice(ChangeAuthorityNotice),
}

pub trait EncodeNotice {
    fn encode_notice(&self) -> EncodedNotice;
}

const ETH_CHAIN_IDENT: &'static [u8] = b"ETH";

// TODO: We might want to make these slightly more efficient
fn encode_addr(raw: &[u8; 20]) -> Vec<u8> {
    let mut res: [u8; 32] = [0; 32];
    res[12..32].copy_from_slice(raw);
    res.to_vec()
}

fn encode_int32(raw: u32) -> Vec<u8> {
    let mut res: [u8; 32] = [0; 32];
    res[28..32].copy_from_slice(&raw.to_be_bytes());
    res.to_vec()
}

fn encode_int128(raw: u128) -> Vec<u8> {
    let mut res: [u8; 32] = [0; 32];
    res[16..32].copy_from_slice(&raw.to_be_bytes());
    res.to_vec()
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
            } => {
                let era_id: Vec<u8> = encode_int32(id.0);
                let era_index: Vec<u8> = encode_int32(id.1);
                let asset_encoded = encode_addr(asset);
                let amount_encoded = encode_int128(*amount);
                let account_encoded = encode_addr(account);

                [
                    ETH_CHAIN_IDENT.to_vec(),
                    era_id,
                    era_index,
                    parent.to_vec(),
                    asset_encoded,
                    amount_encoded,
                    account_encoded,
                ]
                .concat()
            }
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
                amount,
                cash_index,
            } => {
                let amount_encoded = encode_int128(*amount); // XXX cast more safely XXX JF: already converted I think
                [
                    ETH_CHAIN_IDENT.to_vec(),
                    encode_int32(id.0),
                    encode_int32(id.1),
                    parent.to_vec(),
                    encode_addr(account),
                    amount_encoded,
                    encode_int128(*cash_index),
                ]
                .concat()
            }
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
                next_cash_yield_start_at,
                next_cash_index,
            } => [
                ETH_CHAIN_IDENT.to_vec(),
                encode_int32(id.0),
                encode_int32(id.1),
                parent.to_vec(),
                encode_int128(*next_cash_yield),
                encode_int128(*next_cash_yield_start_at),
                encode_int128(*next_cash_index),
            ]
            .concat(),
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
                amount,
            } => {
                let amount_encoded = encode_int128(*amount); // XXX cast more safely XXX JF: already converted I think
                [
                    ETH_CHAIN_IDENT.to_vec(),
                    encode_int32(id.0),
                    encode_int32(id.1),
                    parent.to_vec(),
                    encode_addr(asset),
                    amount_encoded,
                ]
                .concat()
            }
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
            } => {
                let authorities_encoded: Vec<Vec<u8>> =
                    new_authorities.iter().map(|x| encode_addr(x)).collect();

                [
                    ETH_CHAIN_IDENT.to_vec(),
                    encode_int32(id.0),
                    encode_int32(id.1),
                    parent.to_vec(),
                    authorities_encoded.concat(),
                ]
                .concat()
            }
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

// TODO: What's a better way to handle which chain to pull?
pub fn sign_notice_chain(notice: &Notice) -> Result<ChainSignature, Reason> {
    match notice {
        Notice::ExtractionNotice(n) => match n {
            ExtractionNotice::Eth { .. } => Ok(ChainSignature::Eth(
                <Ethereum as Chain>::sign_message(&notice.encode_notice())?,
            )),
        },
        Notice::CashExtractionNotice(n) => match n {
            CashExtractionNotice::Eth { .. } => Ok(ChainSignature::Eth(
                <Ethereum as Chain>::sign_message(&notice.encode_notice())?,
            )),
        },
        Notice::FutureYieldNotice(n) => match n {
            FutureYieldNotice::Eth { .. } => Ok(ChainSignature::Eth(
                <Ethereum as Chain>::sign_message(&notice.encode_notice())?,
            )),
        },
        Notice::SetSupplyCapNotice(n) => match n {
            SetSupplyCapNotice::Eth { .. } => Ok(ChainSignature::Eth(
                <Ethereum as Chain>::sign_message(&notice.encode_notice())?,
            )),
        },
        Notice::ChangeAuthorityNotice(n) => match n {
            ChangeAuthorityNotice::Eth { .. } => Ok(ChainSignature::Eth(
                <Ethereum as Chain>::sign_message(&notice.encode_notice())?,
            )),
        },
    }
}

// TODO: What's a better way to handle which chain to use?
pub fn get_signer_key_for_notice(notice: &Notice) -> Result<ChainAccount, Reason> {
    match notice {
        Notice::ExtractionNotice(n) => match n {
            ExtractionNotice::Eth { .. } => {
                Ok(ChainAccount::Eth(<Ethereum as Chain>::signer_address()?))
            }
        },
        Notice::CashExtractionNotice(n) => match n {
            CashExtractionNotice::Eth { .. } => {
                Ok(ChainAccount::Eth(<Ethereum as Chain>::signer_address()?))
            }
        },
        Notice::FutureYieldNotice(n) => match n {
            FutureYieldNotice::Eth { .. } => {
                Ok(ChainAccount::Eth(<Ethereum as Chain>::signer_address()?))
            }
        },
        Notice::SetSupplyCapNotice(n) => match n {
            SetSupplyCapNotice::Eth { .. } => {
                Ok(ChainAccount::Eth(<Ethereum as Chain>::signer_address()?))
            }
        },
        Notice::ChangeAuthorityNotice(n) => match n {
            ChangeAuthorityNotice::Eth { .. } => {
                Ok(ChainAccount::Eth(<Ethereum as Chain>::signer_address()?))
            }
        },
    }
}

/// Type for the status of a notice on the queue.
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum NoticeStatus {
    Missing,
    Pending {
        signature_pairs: ChainSignatureList,
        notice: Notice,
    },
    Done,
}

impl From<Notice> for NoticeStatus {
    fn from(notice: Notice) -> Self {
        NoticeStatus::Pending {
            signature_pairs: default_notice_signatures(&notice),
            notice,
        }
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

    #[test]
    fn test_encodes_extraction_notice() {
        let notice = Notice::ExtractionNotice(ExtractionNotice::Eth {
            id: NoticeId(80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [3u8; 32],
            asset: [2u8; 20],
            amount: 50,
            account: [1u8; 20],
        });

        let expected = [
            69, 84, 72, // chainType::ETH
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 80, // eraID
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, // eraIndex
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, // parent
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, // asset
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 50, // amount
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, // account
        ];
        let encoded = notice.encode_notice();
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_encodes_cash_extraction_notice() {
        let notice = Notice::CashExtractionNotice(CashExtractionNotice::Eth {
            id: NoticeId(80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [3u8; 32],
            account: [1u8; 20],
            amount: 55,
            cash_index: 75u128,
        });

        let expected = [
            69, 84, 72, // chainType::ETH
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 80, // eraID
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, // eraIndex
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, // parent
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, // account
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 55, // amount
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 75, // cash yield index
        ];

        let encoded = notice.encode_notice();
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_encodes_future_yield_notice() {
        let notice = Notice::FutureYieldNotice(FutureYieldNotice::Eth {
            id: NoticeId(80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [5u8; 32],
            next_cash_yield: 700u128,
            next_cash_yield_start_at: 200u128,
            next_cash_index: 400u128,
        });

        let expected = [
            69, 84, 72, // chainType::ETH
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 80, // eraID
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, // eraIndex
            5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
            5, 5, 5, // parent
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 2, 188, // next cash yield
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 200, // next cash yield start at
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 144, // next cash yield index
        ];

        let encoded = notice.encode_notice();
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_encodes_set_supply_cap_notice() {
        let notice = Notice::SetSupplyCapNotice(SetSupplyCapNotice::Eth {
            id: NoticeId(80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [3u8; 32],
            asset: [70u8; 20],
            amount: 60,
        });

        let expected = [
            69, 84, 72, // chainType::ETH
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 80, // eraID
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, // eraIndex
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, // parent
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 70, 70, 70, 70, 70, 70, 70, 70, 70, 70, 70, 70, 70,
            70, 70, 70, 70, 70, 70, 70, // asset
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 60, // amount
        ];

        let encoded = notice.encode_notice();
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_encodes_new_authorities_notice() {
        let notice = Notice::ChangeAuthorityNotice(ChangeAuthorityNotice::Eth {
            id: NoticeId(80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [3u8; 32],
            new_authorities: vec![[6u8; 20], [7u8; 20], [8u8; 20]],
        });

        let expected = [
            69, 84, 72, // chainType::ETH
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 80, // eraID
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, // eraIndex
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            3, 3, 3, // parent
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6,
            6, 6, 6, // first authority
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, // second
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
            8, 8, 8, // third
        ];

        let encoded = notice.encode_notice();
        assert_eq!(encoded, expected);
    }
}
