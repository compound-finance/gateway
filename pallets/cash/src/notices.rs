// XXX not used but keep until merge with Coburn
use super::{
    account::{AccountIdent, ChainIdent},
    amount::Amount,
};
use codec::{Decode, Encode};
use ethabi;
use frame_support::debug;
use num_traits::ToPrimitive;
use secp256k1;
use our_std::{vec::Vec, RuntimeDebug};
use tiny_keccak::Hasher;

// XXX
pub type Message = Vec<u8>;
pub type Signature = Vec<u8>;
pub type Asset = Vec<u8>; // XXX this should be a tuple like account
pub type EthHash = [u8; 32];

pub type GenerationId = u32;
pub type WithinGenerationId = u32;
pub type NoticeId = (GenerationId, WithinGenerationId);

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct NoticePayload {
    // id: Vec<u8>,
    pub msg: Message,
    pub sig: Signature,
    pub signer: AccountIdent,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum Notice {
    ExtractionNotice {
        chain: ChainIdent,
        id: NoticeId,
        parent: EthHash, // XXX how to parameterize per chain? or generic trait?
        asset: Asset,
        account: AccountIdent,
        amount: Amount,
    },
}

pub fn debug(notice: &Notice) {
    match notice {
        Notice::ExtractionNotice {
            id: _,
            chain: _,
            parent: _,
            asset,
            account,
            amount,
        } => {
            // TODO: safer decoding of the amount
            let x = amount.mantissa.to_u128().unwrap();
            debug::native::info!("notice amount is: {:?}", amount);
        }
    }
}

fn encode(notice: Notice) -> Vec<u8> {
    match notice {
        Notice::ExtractionNotice {
            chain: _,
            id: _,
            parent: _,
            asset,
            account,
            amount,
        } => {
            // TODO: safer decoding of the amount
            let x = amount.mantissa.to_u128().unwrap();

            ethabi::encode(&[
                ethabi::token::Token::FixedBytes(asset.into()),
                ethabi::token::Token::FixedBytes(account.account.into()),
                ethabi::token::Token::Int(x.into()),
            ])
        }
    }
}

pub fn to_payload(notice: Notice) -> NoticePayload {
    let message = encode(notice);
    // TODO: do signer by chain
    let signer = "0x6a72a2f14577D9Cd0167801EFDd54a07B40d2b61"
        .as_bytes()
        .to_vec();
    NoticePayload {
        // id: move id,
        sig: sign(&message),
        msg: message.to_vec(),
        signer: AccountIdent {
            chain: ChainIdent::Eth,
            account: signer,
        },
    }
}

/// Helper function to quickly run keccak in the Ethereum-style
fn keccak(input: Vec<u8>) -> EthHash {
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
