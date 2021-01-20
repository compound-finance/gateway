use crate::eth_decode_hex;
use crate::no_std::*;
use secp256k1::SecretKey;
use sp_core::ecdsa::Pair as EcdsaPair;
use sp_core::sp_std::convert::TryInto;
use sp_core::Pair;
use std::collections::hash_map::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

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

impl From<&str> for KeyId {
    fn from(source: &str) -> Self {
        KeyId {
            data: source.into(),
        }
    }
}

impl Into<String> for KeyId {
    fn into(self) -> String {
        self.data
    }
}

impl Into<String> for &KeyId {
    fn into(self) -> String {
        self.data.clone()
    }
}

impl KeyId {
    pub fn from_utf8(source: Vec<u8>) -> Result<KeyId, CryptoError> {
        let data = String::from_utf8(source).map_err(|_| CryptoError::InvalidKeyId)?;
        Ok(KeyId { data })
    }
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

/// A helper function to sign a message in the style of ethereum
///
/// Reference implementation https://github.com/MaiaVictor/eth-lib/blob/d959c54faa1e1ac8d474028ed1568c5dce27cc7a/src/account.js#L55
/// This is called by web3.js https://github.com/ethereum/web3.js/blob/27c9679766bb4a965843e9bdaea575ea706202f1/packages/web3-eth-accounts/package.json#L18
fn eth_sign(message: &[u8], private_key: &SecretKey) -> Vec<u8> {
    let hashed = eth_keccak_for_signature(message);
    // todo: there is something in this function that says "it is ok for the message to overflow.." that seems bad.
    let message = secp256k1::Message::parse(&hashed);
    let (sig, recovery) = secp256k1::sign(&message, &private_key);
    let recovery_term = recovery.serialize() + ETH_ADD_TO_V;
    let mut sig: Vec<u8> = sig.serialize().into();
    // sig.extend_from_slice(&[0; 31]); // need to pad out for eth_abi
    sig.push(recovery_term);
    sig
}

/// In memory keyring
pub struct InMemoryKeyring {
    /// for now only support ECDSA with curve secp256k1
    keys: HashMap<String, EcdsaPair>,
}

/// The in memory keyring is designed for use in development and not encouraged for use in
/// production. Please use the HSM keyring for production use.
impl InMemoryKeyring {
    /// Create a new in memory keyring
    pub fn new() -> InMemoryKeyring {
        InMemoryKeyring {
            keys: HashMap::new(),
        }
    }

    pub fn new_keyring() -> Box<dyn Keyring> {
        Box::new(Self::new())
    }

    /// Add a key to the keyring with the given key id
    pub fn add(self: &mut Self, key_id: &KeyId, pair: EcdsaPair) {
        self.keys.insert(key_id.data.clone(), pair);
    }

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

    /// Get the eth address (bytes) associated with the given key id.
    fn get_eth_address(self: &Self, key_id: &KeyId) -> Result<Vec<u8>, CryptoError> {
        let public_key = self.get_public_key(key_id)?;
        Ok(public_key_bytes_to_eth_address(&public_key))
    }
}

/// Implement the keyring for the in memory keyring. This allows us to use in memory or HSM
/// mode downstream.
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
        let private = self.get_private_key(key_id)?;
        // could not call serialize from the keypair so I had to re-derive the public key here
        let public = secp256k1::PublicKey::from_secret_key(&private);
        Ok(public_key_to_bytes(public))
    }
}

type ThreadSafeKeyring = dyn Keyring + Send + Sync;

lazy_static::lazy_static! {
    pub static ref KEYRING: Mutex<Arc<ThreadSafeKeyring>> = Mutex::new(Arc::new(dev_keyring()));
}

const ETH_PRIVATE_KEY_ENV_VAR: &str = "ETH_KEY";

const ETH_PRIVATE_KEY_DEFAULT_VALUE: &str =
    "50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374";

/// For dev - get the eth private key from an environment variable
fn get_eth_private_key_from_environment_variable() -> Result<EcdsaPair, CryptoError> {
    let eth_private_key_string = std::env::var(ETH_PRIVATE_KEY_ENV_VAR)
        .map_err(|_| CryptoError::EnvironmentVariablePrivateKeyNotSet)?;
    if eth_private_key_string.len() == 0 {
        return Err(CryptoError::EnvironmentVariablePrivateKeyNotSet);
    }

    let eth_private_key_bytes = eth_decode_hex(&eth_private_key_string)
        .map_err(|_| CryptoError::EnvironmentVariableHexDecodeFailed)?;
    // note - seed is a misnomer - it is actually the private key :(
    let pair = EcdsaPair::from_seed_slice(&eth_private_key_bytes)
        .map_err(|_| CryptoError::EnvironmentVariableInvalidSeed)?;

    Ok(pair)
}

/// Sets up the development keyring, an in memory keyring loaded with the default key
/// for signing messages headed to ethereum.
///
/// WARNING - This function will panic whenever a bad private key is set in the environment
/// variable. That is "ok" because it should only be used during boot.
pub fn dev_keyring() -> InMemoryKeyring {
    let mut keyring = InMemoryKeyring::new();
    let eth_key_id: KeyId = ETH_KEY_ID_ENV_VAR_DEV_DEFAULT.into();

    let pair = match get_eth_private_key_from_environment_variable() {
        Ok(pair) => pair,
        Err(CryptoError::EnvironmentVariablePrivateKeyNotSet) => {
            // fall back to the default value used in unit tests and used on the testnet
            let default_private_key_hex = hex::decode(ETH_PRIVATE_KEY_DEFAULT_VALUE).unwrap();
            // note - seed is a misnomer - it is actually the private key :(
            EcdsaPair::from_seed_slice(&default_private_key_hex).unwrap()
        }
        Err(err) => panic!(
            "Error while decoding private key material for eth key! {:?}",
            err
        ),
    };
    keyring.add(&eth_key_id, pair);

    keyring
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestCase {
        address: String,
        private_key: String,
        data: String,
        signature: String,
    }

    /// These test cases come from web3.js
    fn get_test_cases() -> Vec<TestCase> {
        vec![
            TestCase {
                address: "0xEB014f8c8B418Db6b45774c326A0E64C78914dC0".into(),
                private_key: "0xbe6383dad004f233317e46ddb46ad31b16064d14447a95cc1d8c8d4bc61c3728".into(),
                data: "Some data".into(),
                signature: "0xa8037a6116c176a25e6fc224947fde9e79a2deaa0dd8b67b366fbdfdbffc01f953e41351267b20d4a89ebfe9c8f03c04de9b345add4a52f15bd026b63c8fb150000000000000000000000000000000000000000000000000000000000000001b".into(),
            }, TestCase {
                address: "0xEB014f8c8B418Db6b45774c326A0E64C78914dC0".into(),
                private_key: "0xbe6383dad004f233317e46ddb46ad31b16064d14447a95cc1d8c8d4bc61c3728".into(),
                data: "Some data!%$$%&@*".into(),
                signature: "0x05252412b097c5d080c994d1ea12abcee6f1cae23feb225517a0b691a66e12866b3f54292f9cfef98f390670b4d010fc4af7fcd46e41d72870602c117b14921c000000000000000000000000000000000000000000000000000000000000001c".into(),
            }, /* TestCase { // for now this test case is excluded because the data are encoded as binary
                address: "0xEB014f8c8B418Db6b45774c326A0E64C78914dC0".into(),
                private_key: "0xbe6383dad004f233317e46ddb46ad31b16064d14447a95cc1d8c8d4bc61c3728".into(),
                data: "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470".into(),
                signature: "0xddd493679d80c9c74e0e5abd256a496dfb31b51cd39ea2c7c9e8a2a07de94a90257107a00d9cb631bacb85b208d66bfa7a80c639536b34884505eff352677dd01c".into(),
            } */
        ]
    }

    /// Helper function to decode 0x... to bytes
    fn eth_decode_hex_unsafe(hex: String) -> Vec<u8> {
        let hex = hex.as_bytes();
        hex::decode(&hex[2..]).unwrap()
    }

    /// Test out eth signature function
    fn test_eth_sign_case(case: TestCase) {
        let private_key = eth_decode_hex_unsafe(case.private_key);
        let message: Vec<u8> = case.data.into();
        let secret_key = SecretKey::parse_slice(&private_key).unwrap();
        let actual_sig = eth_sign(&message, &secret_key);
        let expected_sig = eth_decode_hex_unsafe(case.signature);
        assert_eq!(actual_sig, expected_sig);
    }

    #[test]
    fn test_eth_sign() {
        // Test cases found in web3js
        // https://github.com/ethereum/web3.js/blob/27c9679766bb4a965843e9bdaea575ea706202f1/test/eth.accounts.sign.js#L7
        get_test_cases().drain(..).for_each(test_eth_sign_case);
    }

    /// Test out eth recover function
    fn test_eth_recover_case(case: TestCase) {
        let message: Vec<u8> = case.data.into();
        let sig = eth_decode_hex_unsafe(case.signature);
        let actual_address = eth_recover(&message, &sig).unwrap();
        let expected_address = eth_decode_hex_unsafe(case.address);
        assert_eq!(actual_address, expected_address);
    }

    #[test]
    fn test_eth_recover() {
        // Test cases found in web3js
        // https://github.com/ethereum/web3.js/blob/27c9679766bb4a965843e9bdaea575ea706202f1/test/eth.accounts.sign.js#L7
        get_test_cases().drain(..).for_each(test_eth_recover_case);
    }

    fn get_test_keyring_from_test_case(case: &TestCase) -> (KeyId, InMemoryKeyring) {
        get_test_keyring(case.private_key.clone())
    }

    pub fn get_test_keyring(private_key: String) -> (KeyId, InMemoryKeyring) {
        let key_id = KeyId {
            data: "hello".into(),
        };
        let mut keyring = InMemoryKeyring::new();
        let private_key_bytes = eth_decode_hex_unsafe(private_key.clone());
        let pair = EcdsaPair::from_seed_slice(&private_key_bytes).unwrap();
        keyring.add(&key_id, pair);

        (key_id, keyring)
    }

    fn test_sign_case(case: TestCase) {
        let (key_id, keyring) = get_test_keyring_from_test_case(&case);
        let message: Vec<u8> = case.data.into();
        let messages = vec![message];
        let sigs = keyring.sign(messages, &key_id).unwrap();
        assert_eq!(sigs.len(), 1);
        let actual_sig = sigs[0].as_ref().unwrap();
        let expected_sig = &eth_decode_hex_unsafe(case.signature);
        assert_eq!(actual_sig, expected_sig);
    }

    #[test]
    fn test_sign() {
        get_test_cases().drain(..).for_each(test_sign_case);
    }

    fn test_public_key_case(case: TestCase) {
        let (key_id, keyring) = get_test_keyring_from_test_case(&case);
        let actual_address = keyring.get_eth_address(&key_id).unwrap();
        let expected_address = eth_decode_hex_unsafe(case.address);
        assert_eq!(actual_address, expected_address);
    }

    #[test]
    fn test_public_key() {
        get_test_cases().drain(..).for_each(test_public_key_case);
    }
}
