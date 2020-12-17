use super::{
    account::{AccountIdent, ChainIdent},
    amount::Amount,
    chains,
};
use codec::{Decode, Encode};
use ethabi;
use num_traits::ToPrimitive;
use secp256k1;
use sp_std::vec::Vec;
use tiny_keccak::Hasher;
use hex::ToHex;

// XXX
pub type Message = Vec<u8>;
pub type Signature = Vec<u8>;
pub type Asset = Vec<u8>; // XXX this should be a tuple like account

pub type Index = u128; // XXX
pub type Rate = u128; // XXX
pub type Timestamp = u32; // XXX

pub type GenerationId = u32;
pub type WithinGenerationId = u32;
pub type NoticeId = (GenerationId, WithinGenerationId);

#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
pub struct NoticePayload {
    // id: Vec<u8>,
    pub msg: Message,
    pub sig: Signature,
    pub signer: AccountIdent,
}

// #[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
// pub enum Notice {
//     ExtractionNotice {
//         chain: ChainIdent,
//         id: NoticeId,
//         parent: EthHash, // XXX how to parameterize per chain? or generic trait?
//         asset: Asset,
//         account: AccountIdent,
//         amount: Amount,
//     },
// }

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub enum Notice<Chain: chains::Chain> {
    ExtractionNotice {
        id: NoticeId,
        parent: Chain::Hash,
        asset: Chain::Asset,
        account: Chain::Account,
        amount: Amount,
    },

    CashExtractionNotice {
        id: NoticeId,
        parent: Chain::Hash,
        account: Chain::Asset,
        amount: Chain::Account,
        cash_yield_index: Index,
    },

    FutureYieldNotice {
        id: NoticeId,
        parent: Chain::Hash,
        next_cash_yield: Rate,
        next_cash_yield_start_at: Timestamp,
        next_cash_yield_index: Index,
    },

    SetSupplyCapNotice {
        id: NoticeId,
        parent: Chain::Hash,
        asset: Chain::Asset,
        amount: Amount,
    },

    ChangeAuthorityNotice {
        id: NoticeId,
        parent: Chain::Hash,
        new_authorities: Vec<Chain::Public>,
    },
}


fn encode_ethereum_notice(notice: Notice<chains::Ethereum>) -> Vec<u8> {
    let chain_ident: Vec<u8> = "ETH".into(); // XX make const?
    let encode_addr = |raw: [u8; 20]| -> Vec<u8> { ethabi::encode(&[ethabi::Token::FixedBytes(raw.into())]) };
   
    // XXX do this better, maybe figure out how to use dyn Into<ethereum_types::U256> ?
    let encode_int32 = |raw: u32| -> Vec<u8> { ethabi::encode(&[ethabi::Token::Int(raw.into())]) };
    let encode_int128 = |raw: u128| -> Vec<u8> { ethabi::encode(&[ethabi::Token::Int(raw.into())]) };
    
    match notice {
        Notice::ExtractionNotice {
            id,
            parent,
            asset: asset_arr,
            account: account_arr,
            amount: amount_tup,
        } => {
            let era_id: Vec<u8> = encode_int32(id.0);
            let era_index: Vec<u8> = encode_int32(id.1);
            let asset = encode_addr(asset_arr);
            let amount = encode_int128(amount_tup.mantissa.to_u128().unwrap().into());// XXX cast more safely
            let account = encode_addr(account_arr);

            [chain_ident, era_id, era_index, parent.into(), asset, amount, account].concat()
        }
        Notice::CashExtractionNotice { .. } => {
            vec![]
        }

        Notice::FutureYieldNotice { .. } => {
            vec![]
        }

        Notice::SetSupplyCapNotice { .. } => {
            vec![]
        }

        Notice::ChangeAuthorityNotice { .. } => {
            vec![]
        }
    }
}

// pub fn to_eth_payload(notice: Notice) -> NoticePayload {
//     let message = encode_ethereum_notice(notice);
//     // TODO: do signer by chain
//     let signer = "0x6a72a2f14577D9Cd0167801EFDd54a07B40d2b61"
//         .as_bytes()
//         .to_vec();
//     NoticePayload {
//         // id: move id,
//         sig: sign(&message),
//         msg: message.to_vec(), // perhaps hex::encode(message)
//         signer: AccountIdent {
//             chain: ChainIdent::Eth,
//             account: signer,
//         },
//     }
// }

/// Helper function to quickly run keccak in the Ethereum-style
fn keccak(input: Vec<u8>) -> [u8; 32] {
    let mut output = [0u8; 32];
    let mut hasher = tiny_keccak::Keccak::v256();
    hasher.update(&input[..]);
    hasher.finalize(&mut output);
    output
}

// TODO: match by chain for signing algorithm or implement as trait
fn sign(message: &Message) -> Signature {
    // TODO: get this from somewhere else
    let not_so_secret: [u8; 32] =
        hex_literal::hex!["50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374"];
    let private_key = secp256k1::SecretKey::parse(&not_so_secret).unwrap();

    let msg = secp256k1::Message::parse(&keccak(message.clone()));
    let x = secp256k1::sign(&msg, &private_key);

    let mut r: [u8; 65] = [0; 65];
    r[0..64].copy_from_slice(&x.0.serialize()[..]);
    r[64] = x.1.serialize();
    sp_core::ecdsa::Signature::from_raw(r).encode()
}

#[cfg(test)]
mod tests {
    use crate::*;
    use sp_std::prelude::*;

    #[test]
    fn test_encodes_extraction_notice() {
        let notice = notices::Notice::ExtractionNotice {
            id: (80, 0), // XXX need to keep state of current gen/within gen for each, also parent
            parent: [3u8; 32],
            asset: [2u8; 20],
            amount: Amount::new_cash(50 as u128),
            account: [1u8; 20],
        };

        let expected = [
            69, 84, 72, // chainType::ETH
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, // eraID
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // eraIndex
            3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // parent
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // asset
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 50, // amount
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 // account
        ];

        let encoded = notices::encode_ethereum_notice(notice);
        assert_eq!(encoded, expected);
    }
}
