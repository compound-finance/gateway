use codec::{Decode, Encode};
use our_std::{convert::TryInto, RuntimeDebug};
use tiny_keccak::Hasher;

use types_derive::Types;

pub type SignatureBytes = [u8; 65];

pub type AddressBytes = [u8; 20];

pub type PublicKeyBytes = [u8; 64];

pub type SignatureBytesWithoutRecovery = [u8; 64];

pub type TaggedPublicKeyBytes = [u8; 65];

pub type HashedMessageBytes = [u8; 32];

/// The crypto error type allows for various failure scenarios
///
/// * The key id provided is unknown
/// * The HSM is not available
/// * The HSM failed to sign this request for some other reason
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug, Types)]
pub enum CryptoError {
    Unknown,
    KeyNotFound,
    KeyringLock,
    InvalidKeyId,
    ParseError,
    RecoverError,
    HSMError,
    EnvironmentVariablePrivateKeyNotSet,
    HexDecodeFailed,
    EnvironmentVariableHexDecodeFailed,
    EnvironmentVariableInvalidSeed,
}

/// The default key id for the eth authority key (l1)
pub const ETH_KEY_ID_ENV_VAR_DEV_DEFAULT: &str = "my_eth_key_id";

/// For compatibility this is required.
pub const ETH_MESSAGE_PREAMBLE: &[u8] = "\x19Ethereum Signed Message:\n".as_bytes();

/// For compatibility this is required.
pub const ETH_ADD_TO_V: u8 = 27u8;

/// Helper function to quickly run keccak in the Ethereum-style
/// This includes the preamble and length ouf output
pub fn eth_keccak_for_signature(input: &[u8], prepend_preamble: bool) -> HashedMessageBytes {
    let mut output = [0u8; 32];
    let mut hasher = tiny_keccak::Keccak::v256();
    if prepend_preamble {
        hasher.update(ETH_MESSAGE_PREAMBLE);
        hasher.update(format!("{}", input.len()).as_bytes());
    }
    hasher.update(input);
    hasher.finalize(&mut output);
    output
}

/// Run keccak256 on the input.
/// This does not include the preamble and length of output.
pub fn keccak(input: &[u8]) -> HashedMessageBytes {
    let mut output = [0u8; 32];
    let mut hasher = tiny_keccak::Keccak::v256();
    hasher.update(&input[..]);
    hasher.finalize(&mut output);
    output
}

pub fn tagged_public_key_to_raw(public_key: TaggedPublicKeyBytes) -> PublicKeyBytes {
    match public_key {
        [_, e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12, e13, e14, e15, e16, e17, e18, e19, e20, e21, e22, e23, e24, e25, e26, e27, e28, e29, e30, e31, e32, e33, e34, e35, e36, e37, e38, e39, e40, e41, e42, e43, e44, e45, e46, e47, e48, e49, e50, e51, e52, e53, e54, e55, e56, e57, e58, e59, e60, e61, e62, e63, e64, e65] => {
            [
                e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12, e13, e14, e15, e16, e17, e18, e19,
                e20, e21, e22, e23, e24, e25, e26, e27, e28, e29, e30, e31, e32, e33, e34, e35,
                e36, e37, e38, e39, e40, e41, e42, e43, e44, e45, e46, e47, e48, e49, e50, e51,
                e52, e53, e54, e55, e56, e57, e58, e59, e60, e61, e62, e63, e64, e65,
            ]
        }
    }
}

pub fn tagged_public_key_slice_to_raw(public_key: &[u8]) -> Result<PublicKeyBytes, CryptoError> {
    match public_key {
        [_, e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12, e13, e14, e15, e16, e17, e18, e19, e20, e21, e22, e23, e24, e25, e26, e27, e28, e29, e30, e31, e32, e33, e34, e35, e36, e37, e38, e39, e40, e41, e42, e43, e44, e45, e46, e47, e48, e49, e50, e51, e52, e53, e54, e55, e56, e57, e58, e59, e60, e61, e62, e63, e64, e65] => {
            Ok([
                *e2, *e3, *e4, *e5, *e6, *e7, *e8, *e9, *e10, *e11, *e12, *e13, *e14, *e15, *e16,
                *e17, *e18, *e19, *e20, *e21, *e22, *e23, *e24, *e25, *e26, *e27, *e28, *e29, *e30,
                *e31, *e32, *e33, *e34, *e35, *e36, *e37, *e38, *e39, *e40, *e41, *e42, *e43, *e44,
                *e45, *e46, *e47, *e48, *e49, *e50, *e51, *e52, *e53, *e54, *e55, *e56, *e57, *e58,
                *e59, *e60, *e61, *e62, *e63, *e64, *e65,
            ])
        }
        _ => Err(CryptoError::ParseError),
    }
}

/// Convert the public key bytes to an ETH address
pub fn public_key_bytes_to_eth_address(public_key: &PublicKeyBytes) -> AddressBytes {
    let public_hash = keccak(public_key); // 32 bytes
    let public_hash_tail = match public_hash {
        [_, _, _, _, _, _, _, _, _, _, _, _, e13, e14, e15, e16, e17, e18, e19, e20, e21, e22, e23, e24, e25, e26, e27, e28, e29, e30, e31, e32] => {
            [
                e13, e14, e15, e16, e17, e18, e19, e20, e21, e22, e23, e24, e25, e26, e27, e28,
                e29, e30, e31, e32,
            ]
        }
    };

    public_hash_tail
}

/// Convert a secp256k1 public key to a "RAW" public key format.
pub fn public_key_to_bytes(public: secp256k1::PublicKey) -> PublicKeyBytes {
    // some tag is added here need to strip it
    let serialized = public.serialize();
    tagged_public_key_to_raw(serialized)
}

/// Convert a secp256k1 public key into an eth address
pub fn public_key_to_eth_address(public: secp256k1::PublicKey) -> AddressBytes {
    let bytes = public_key_to_bytes(public);
    let address = public_key_bytes_to_eth_address(&bytes);

    address
}

const ETH_SIGNATURE_PADDED_RECOVERY_LENGTH: usize = 96;
const ETH_SIGNATURE_UNPADDED_RECOVERY_LENGTH: usize = 65;

pub fn eth_signature_from_bytes(bytes: &[u8]) -> Result<SignatureBytes, CryptoError> {
    if bytes.len() != ETH_SIGNATURE_PADDED_RECOVERY_LENGTH
        && bytes.len() != ETH_SIGNATURE_UNPADDED_RECOVERY_LENGTH
    {
        return Err(CryptoError::ParseError);
    }

    match bytes {
        [e1, e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12, e13, e14, e15, e16, e17, e18, e19, e20, e21, e22, e23, e24, e25, e26, e27, e28, e29, e30, e31, e32, e33, e34, e35, e36, e37, e38, e39, e40, e41, e42, e43, e44, e45, e46, e47, e48, e49, e50, e51, e52, e53, e54, e55, e56, e57, e58, e59, e60, e61, e62, e63, e64, .., last] => {
            Ok([
                *e1, *e2, *e3, *e4, *e5, *e6, *e7, *e8, *e9, *e10, *e11, *e12, *e13, *e14, *e15,
                *e16, *e17, *e18, *e19, *e20, *e21, *e22, *e23, *e24, *e25, *e26, *e27, *e28, *e29,
                *e30, *e31, *e32, *e33, *e34, *e35, *e36, *e37, *e38, *e39, *e40, *e41, *e42, *e43,
                *e44, *e45, *e46, *e47, *e48, *e49, *e50, *e51, *e52, *e53, *e54, *e55, *e56, *e57,
                *e58, *e59, *e60, *e61, *e62, *e63, *e64, *last,
            ])
        }
        _ => Err(CryptoError::ParseError),
    }
}

pub fn bytes_to_eth_hex_string(message: &[u8]) -> String {
    format!("0x{}", hex::encode(message))
}

pub fn eth_decode_hex(message: &str) -> Result<Vec<u8>, CryptoError> {
    eth_decode_hex_ascii(message.as_bytes())
}

const ETH_HEX_PREFIX: &str = "0x";

pub fn eth_decode_hex_ascii(message: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if &message[..2] != ETH_HEX_PREFIX.as_bytes() {
        hex::decode(message).map_err(|_| CryptoError::HexDecodeFailed)
    } else {
        hex::decode(&message[2..]).map_err(|_| CryptoError::HexDecodeFailed)
    }
}

pub fn str_to_address(addr: &str) -> Option<[u8; 20]> {
    if addr.len() == 42 && &addr[0..2] == "0x" {
        if let Ok(bytes) = hex::decode(&addr[2..42]) {
            if let Ok(address) = bytes.try_into() {
                return Some(address);
            }
        }
    }
    return None;
}

pub fn address_string(address: &[u8; 20]) -> String {
    format!("0x{}", hex::encode(address))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_eth_decode_hex_ascii_fails_on_unicode() {
        let cases = vec!["0💘"];
        for case in cases {
            assert!(case.len() > 2);
            assert!(eth_decode_hex_ascii(case.as_bytes()).is_err());
        }
    }
}
