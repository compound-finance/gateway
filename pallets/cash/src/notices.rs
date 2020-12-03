use tiny_keccak::Hasher;
use sp_std::vec::Vec;
use secp256k1;
use ethabi;

pub type Message = Vec<u8>;
pub type Signature = Vec<u8>;
pub type Address = Vec<u8>;
pub type Asset = (Chain, Address);
pub type Account = (Chain, Address);
pub type Amount = Vec<u8>;
pub type Timestamp = u32;
pub type Index = u32;
pub type Rate = u32;

pub struct NoticePayload {
    // id: Vec<u8>,
    msg: Vec<u8>,
    sig: Vec<u8>, 
    public: Address,
}

pub trait Notice {
    fn encode(&self) -> Message;
}

pub enum Chain {Eth}

pub struct ExtractionNotice {
    asset: Asset,
    account: Account,
    amount: Address,
}

impl Notice for ExtractionNotice {
    fn encode (&self) -> Vec<u8> {
        ethabi::encode(&[
            ethabi::token::Token::FixedBytes(self.asset.1.clone().into()),
            ethabi::token::Token::FixedBytes(self.account.1.clone().into()),
            ethabi::token::Token::Int(self.amount.clone().into()),
        ])
    }
}

/// Helper function to quickly run keccak in the Ethereum-style
fn keccak(input: Vec<u8>) -> [u8; 32] {
    let mut output = [0u8; 32];
    let mut hasher = tiny_keccak::Keccak::v256();
    hasher.update(&input[..]);
    hasher.finalize(&mut output);
    output
}

// TODO: match by chain for signing algorotih or implement as trait
pub fn sign(message : &Message) -> Signature {
    // TODO: get this from somewhere else
    let not_so_secret: [u8; 32] = hex_literal::hex!["50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374"];
    let private_key = secp256k1::SecretKey::parse(&not_so_secret).unwrap();

    let msg = secp256k1::Message::parse(&keccak(message.clone()));
    let x = secp256k1::sign(&msg, &private_key);

    let mut r: [u8; 65] = [0; 65];
    r[0..64].copy_from_slice(&x.0.serialize()[..]);
    r[64] = x.1.serialize();
    sp_core::ecdsa::Signature::from_raw(r).encode()
}
