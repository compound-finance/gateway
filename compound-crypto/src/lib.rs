use secp256k1::SecretKey;
use sp_core::ecdsa::Pair as EcdsaPair;
use sp_core::{Pair, Public};
use std::collections::hash_map::HashMap;
use tiny_keccak::Hasher;

/// The crypto module for compound chain.
///
/// This entire crate is STD and designed to be exposed via
/// a runtime interface in the runtime-interfaces crate.
///
/// The concept here is to generically wrap most HSM systems. The generic HSM system will
/// maintain some keys externally identified by some string. The Key ID identifies the
/// key and is what is used to pass to the HSM along with the payload to sign.
///
/// In AWS the KeyID is most likely the key's ARN but it is also likely that the key's
/// alias could be used if it has one.

/// The Key ID type identifies a Key in the HSM
#[derive(Clone)]
pub struct KeyId {
    data: String,
}

/// The crypto error type allows for various failure scenarios
///
/// * The key id provided is unknown
/// * The HSM is not available
/// * The HSM failed to sign this request for some other reason
pub enum CryptoError {
    KeyNotFound,
    Unknown,
}

/// A keyring abstraction for HSMs
pub trait Keyring {
    /// Batch sign messages with the given key
    ///
    /// It is important that this interface is batch because likely during the implementation
    /// we will want to use async IO or at the very least threading to make multiple simultaneous
    /// calls to the HSM to sign the payloads with the same key. This is a very common flow for
    /// the runtime. The idea is that if the keyring is exposed via a runtime-interface
    /// the implementation of `sign` on the node side of the interface can use async IO and/or
    /// threading to implement batched sign requests whereas within the runtime it will appear
    /// to be a completely synchronous request. This is important to maintain the ability
    /// to upgrade the OCW code via the WASM blob in the forkless upgrade process.
    ///
    /// Because this is a batch method, there is a Vec of Results which correspond to each signing
    /// request for each message in order. All of that is wrapped in a result in case we cannot find
    /// the key at all.
    fn sign(
        self: &Self,
        messages: Vec<Vec<u8>>,
        key_id: &KeyId,
    ) -> Result<Vec<Result<Vec<u8>, CryptoError>>, CryptoError>;

    /// Get the public key data for the key id provided.
    /// Fails whenever the key_id is not found in the keyring.
    fn get_public_key(self: &Self, key_id: &KeyId) -> Result<Vec<u8>, CryptoError>;
}

/// In memory keyring
pub struct InMemoryKeyring {
    /// for now only support ECDSA with curve secp256k1
    keys: HashMap<String, EcdsaPair>,
}

/// Helper function to quickly run keccak in the Ethereum-style
fn keccak(input: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    let mut hasher = tiny_keccak::Keccak::v256();
    hasher.update(input);
    hasher.finalize(&mut output);
    output
}

/// A helper function to sign a message in the style of ethereum
///
/// Does not add the Ethereum prelude, perhaps it should?
fn eth_sign(message: &[u8], private_key: &SecretKey) -> Vec<u8> {
    let hashed = keccak(message);
    // todo: there is something in this function that says "it is ok for the message to overflow.." that seems bad.
    let message = secp256k1::Message::parse(&hashed);
    let (sig, _) = secp256k1::sign(&message, &private_key);
    let sig: Vec<u8> = sig.serialize().into();
    sig
}

impl InMemoryKeyring {
    /// Get the keypair associated with the given key id
    fn get_keypair(self: &Self, key_id: &KeyId) -> Result<&EcdsaPair, CryptoError> {
        self.keys.get(&key_id.data).ok_or(CryptoError::KeyNotFound)
    }

    /// Get the private key associated with the key ID (for signing)
    fn get_private_key(self: &Self, key_id: &KeyId) -> Result<SecretKey, CryptoError> {
        let keypair = self.get_keypair(key_id)?;
        // note - it seems to me that this method is misnamed and this actually provides
        // the private key
        let private_key = keypair.seed();
        let private_key =
            secp256k1::SecretKey::parse(&private_key).map_err(|_| CryptoError::Unknown)?;

        Ok(private_key)
    }
}

impl Keyring for InMemoryKeyring {
    /// Sign the messages with the given Key ID
    fn sign(
        self: &Self,
        messages: Vec<Vec<u8>>,
        key_id: &KeyId,
    ) -> Result<Vec<Result<Vec<u8>, CryptoError>>, CryptoError> {
        let private_key = self.get_private_key(key_id)?;

        let result = messages
            .iter()
            .map(|message| Ok(eth_sign(message, &private_key)))
            .collect();

        Ok(result)
    }

    /// Get the public key associated with the given key id.
    fn get_public_key(self: &Self, key_id: &KeyId) -> Result<Vec<u8>, CryptoError> {
        let keypair = self.get_keypair(key_id)?;
        let public = keypair.public().to_raw_vec();
        Ok(public)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign() {
        // todo: write tests for dev setup
    }

    // note- took keccak test cases from tiny_keccak crate

    #[test]
    fn empty_keccak() {
        let mut output = [0; 32];
        let input = [0u8; 0];

        let expected = b"\
            \xc5\xd2\x46\x01\x86\xf7\x23\x3c\x92\x7e\x7d\xb2\xdc\xc7\x03\xc0\
            \xe5\x00\xb6\x53\xca\x82\x27\x3b\x7b\xfa\xd8\x04\x5d\x85\xa4\x70\
        ";

        let output = keccak(&input);
        assert_eq!(expected, &output);
    }

    #[test]
    fn string_keccak_256() {
        let mut input: [u8; 5] = [1, 2, 3, 4, 5];
        let expected = b"\
            \x7d\x87\xc5\xea\x75\xf7\x37\x8b\xb7\x01\xe4\x04\xc5\x06\x39\x16\
            \x1a\xf3\xef\xf6\x62\x93\xe9\xf3\x75\xb5\xf1\x7e\xb5\x04\x76\xf4\
        ";
        let output = keccak(&input[0..5]);
        assert_eq!(expected, &output);
    }

    #[test]
    fn test_eth_sign() {
        // todo: need some good test data for this one. Somewhat unclear to me how best to obtain it
    }
}
