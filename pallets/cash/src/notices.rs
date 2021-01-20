use crate::{
    chains::{Chain, Ethereum},
    core::{Account, Asset},
    GenericMsg,
};
use codec::{Decode, Encode};
use our_std::{vec::Vec, RuntimeDebug};

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
        cash_yield_index: <Ethereum as Chain>::MulIndex,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum FutureYieldNotice {
    Eth {
        id: NoticeId,
        parent: <Ethereum as Chain>::Hash,
        next_cash_yield: <Ethereum as Chain>::Rate,
        next_cash_yield_start_at: <Ethereum as Chain>::Timestamp,
        next_cash_yield_index: <Ethereum as Chain>::MulIndex,
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
    fn encode_ethereum_notice(&self) -> GenericMsg;
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
    fn encode_ethereum_notice(&self) -> GenericMsg {
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
    fn encode_ethereum_notice(&self) -> GenericMsg {
        match self {
            CashExtractionNotice::Eth {
                id,
                parent,
                account,
                amount,
                cash_yield_index,
            } => {
                let amount_encoded = encode_int128(*amount); // XXX cast more safely XXX JF: already converted I think
                [
                    ETH_CHAIN_IDENT.to_vec(),
                    encode_int32(id.0),
                    encode_int32(id.1),
                    parent.to_vec(),
                    encode_addr(account),
                    amount_encoded,
                    encode_int128(*cash_yield_index),
                ]
                .concat()
            }
        }
    }
}

impl EncodeNotice for FutureYieldNotice {
    fn encode_ethereum_notice(&self) -> GenericMsg {
        match self {
            FutureYieldNotice::Eth {
                id,
                parent,
                next_cash_yield,
                next_cash_yield_start_at,
                next_cash_yield_index,
            } => [
                ETH_CHAIN_IDENT.to_vec(),
                encode_int32(id.0),
                encode_int32(id.1),
                parent.to_vec(),
                encode_int128(*next_cash_yield),
                encode_int128(*next_cash_yield_start_at),
                encode_int128(*next_cash_yield_index),
            ]
            .concat(),
        }
    }
}

impl EncodeNotice for SetSupplyCapNotice {
    fn encode_ethereum_notice(&self) -> GenericMsg {
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
    fn encode_ethereum_notice(&self) -> GenericMsg {
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
    fn encode_ethereum_notice(&self) -> GenericMsg {
        match self {
            Notice::ExtractionNotice(n) => n.encode_ethereum_notice(),
            Notice::CashExtractionNotice(n) => n.encode_ethereum_notice(),
            Notice::FutureYieldNotice(n) => n.encode_ethereum_notice(),
            Notice::SetSupplyCapNotice(n) => n.encode_ethereum_notice(),
            Notice::ChangeAuthorityNotice(n) => n.encode_ethereum_notice(),
        }
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
        let encoded = notice.encode_ethereum_notice();
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_encodes_cash_extraction_notice() {
        let notice = Notice::CashExtractionNotice(CashExtractionNotice::Eth {
            id: NoticeId(80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [3u8; 32],
            account: [1u8; 20],
            amount: 55,
            cash_yield_index: 75u128,
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

        let encoded = notice.encode_ethereum_notice();
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_encodes_future_yield_notice() {
        let notice = Notice::FutureYieldNotice(FutureYieldNotice::Eth {
            id: NoticeId(80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [5u8; 32],
            next_cash_yield: 700u128,
            next_cash_yield_start_at: 200u128,
            next_cash_yield_index: 400u128,
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

        let encoded = notice.encode_ethereum_notice();
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

        let encoded = notice.encode_ethereum_notice();
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

        let encoded = notice.encode_ethereum_notice();
        assert_eq!(encoded, expected);
    }
}
