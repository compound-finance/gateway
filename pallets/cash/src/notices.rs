use crate::chains::{Chain, Ethereum};
use codec::{Decode, Encode};
use our_std::{vec::Vec, RuntimeDebug};

// XXX
pub type Message = Vec<u8>;
pub type Signature = Vec<u8>; // XXX bunch of Signature types now, rename to avoid confusion?
pub type Signatures = Vec<Signature>;

pub type EraId = u32;
pub type EraIndex = u32;
pub type NoticeId = (EraId, EraIndex); // XXX make totally ordered trait

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum Notice<C: Chain> {
    ExtractionNotice {
        id: NoticeId,
        parent: C::Hash,
        asset: C::Asset,
        account: C::Account,
        amount: C::Amount,
    },

    CashExtractionNotice {
        id: NoticeId,
        parent: C::Hash,
        account: C::Account,
        amount: C::Amount,
        cash_yield_index: C::Index,
    },

    FutureYieldNotice {
        id: NoticeId,
        parent: C::Hash,
        next_cash_yield: C::Rate,
        next_cash_yield_start_at: C::Timestamp,
        next_cash_yield_index: C::Index,
    },

    SetSupplyCapNotice {
        id: NoticeId,
        parent: C::Hash,
        asset: C::Asset,
        amount: C::Amount,
    },

    ChangeAuthorityNotice {
        id: NoticeId,
        parent: C::Hash,
        new_authorities: Vec<C::Address>,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum NoticeStatus<C: Chain> {
    Missing,
    Pending {
        signers: crate::ValidatorSet,
        signatures: Signatures,
        notice: Notice<C>,
    },
    Done,
}

impl<C: Chain> Notice<C> {
    pub fn id(&self) -> NoticeId {
        match self {
            Notice::ExtractionNotice { id, .. } => *id,
            Notice::CashExtractionNotice { id, .. } => *id,
            Notice::FutureYieldNotice { id, .. } => *id,
            Notice::SetSupplyCapNotice { id, .. } => *id,
            Notice::ChangeAuthorityNotice { id, .. } => *id,
        }
    }
}

pub fn encode_ethereum_notice(notice: Notice<Ethereum>) -> Message {
    // XXX
    let chain_ident: Vec<u8> = "ETH".into(); // XX make const?

    let encode_addr = |raw: [u8; 20]| -> Vec<u8> {
        let mut res: [u8; 32] = [0; 32];
        res[12..32].clone_from_slice(&raw);
        res.to_vec()
    };

    let encode_int32 = |raw: u32| -> Vec<u8> {
        let mut res: [u8; 32] = [0; 32];
        res[28..32].clone_from_slice(&raw.to_be_bytes());
        res.to_vec()
    };

    let encode_int128 = |raw: u128| -> Vec<u8> {
        let mut res: [u8; 32] = [0; 32];
        res[16..32].clone_from_slice(&raw.to_be_bytes());
        res.to_vec()
    };

    match notice {
        Notice::ExtractionNotice {
            id,
            parent,
            asset,
            account,
            amount: amount,
        } => {
            let era_id: Vec<u8> = encode_int32(id.0);
            let era_index: Vec<u8> = encode_int32(id.1);
            let asset_encoded = encode_addr(asset);
            let amount_encoded = encode_int128(amount);
            let account_encoded = encode_addr(account);

            [
                chain_ident,
                era_id,
                era_index,
                parent.into(),
                asset_encoded,
                amount_encoded,
                account_encoded,
            ]
            .concat()
        }
        Notice::CashExtractionNotice {
            id,
            parent,
            account,
            amount,
            cash_yield_index,
        } => {
            let amount_encoded = encode_int128(amount); // XXX cast more safely XXX JF: already converted I think
            [
                chain_ident,
                encode_int32(id.0),
                encode_int32(id.1),
                parent.into(),
                encode_addr(account),
                amount_encoded,
                encode_int128(cash_yield_index),
            ]
            .concat()
        }

        Notice::FutureYieldNotice {
            id,
            parent,
            next_cash_yield,          //: Rate,
            next_cash_yield_start_at, //: Timestamp,
            next_cash_yield_index,    //: Index,
        } => [
            chain_ident,
            encode_int32(id.0),
            encode_int32(id.1),
            parent.into(),
            encode_int128(next_cash_yield),
            encode_int128(next_cash_yield_start_at),
            encode_int128(next_cash_yield_index),
        ]
        .concat(),

        Notice::SetSupplyCapNotice {
            id,     //: NoticeId,
            parent, //: C::Hash,
            asset,  //: C::Asset,
            amount, //: Amount,
        } => {
            let amount_encoded = encode_int128(amount); // XXX cast more safely XXX JF: already converted I think

            [
                chain_ident,
                encode_int32(id.0),
                encode_int32(id.1),
                parent.into(),
                encode_addr(asset),
                amount_encoded,
            ]
            .concat()
        }

        Notice::ChangeAuthorityNotice {
            id,              //: NoticeId,
            parent,          //: C::Hash,
            new_authorities, //: Vec<C::PublicKey>
        } => {
            let authorities_encoded: Vec<Vec<u8>> = new_authorities
                .iter()
                .map(|x| encode_addr(x.clone()))
                .collect();
            [
                chain_ident,
                encode_int32(id.0),
                encode_int32(id.1),
                parent.into(),
                authorities_encoded.concat(),
            ]
            .concat()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_encodes_extraction_notice() {
        let notice = notices::Notice::ExtractionNotice::<Ethereum> {
            id: (80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [3u8; 32],
            asset: [2u8; 20],
            amount: 50,
            account: [1u8; 20],
        };

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
        let encoded = notices::encode_ethereum_notice(notice);
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_encodes_cash_extraction_notice() {
        let notice = notices::Notice::CashExtractionNotice {
            id: (80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [3u8; 32],
            account: [1u8; 20],
            amount: 55,
            cash_yield_index: 75u128,
        };

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

        let encoded = notices::encode_ethereum_notice(notice);
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_encodes_future_yield_notice() {
        let notice = notices::Notice::FutureYieldNotice {
            id: (80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [5u8; 32],
            next_cash_yield: 700u128,
            next_cash_yield_start_at: 200u128,
            next_cash_yield_index: 400u128,
        };

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

        let encoded = notices::encode_ethereum_notice(notice);
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_encodes_set_supply_cap_notice() {
        let notice = notices::Notice::SetSupplyCapNotice {
            id: (80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [3u8; 32],
            asset: [70u8; 20],
            amount: 60,
        };

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

        let encoded = notices::encode_ethereum_notice(notice);
        assert_eq!(encoded, expected);
    }

    #[test]
    fn test_encodes_new_authorities_notice() {
        let notice = notices::Notice::ChangeAuthorityNotice {
            id: (80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [3u8; 32],
            new_authorities: vec![[6u8; 20], [7u8; 20], [8u8; 20]],
        };

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

        let encoded = notices::encode_ethereum_notice(notice);
        assert_eq!(encoded, expected);
    }
}
