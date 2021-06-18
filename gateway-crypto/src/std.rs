use crate::aws_kms;
use crate::dev_keyring;
use crate::no_std::*;
use secp256k1::SecretKey;
use sp_core::ecdsa::Pair as EcdsaPair;
use std::collections::hash_map::HashMap;

/// The crypto module for gateway.
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

impl From<String> for KeyId {
    fn from(source: String) -> Self {
        KeyId { data: source }
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
        messages: Vec<&[u8]>,
        key_id: &KeyId,
    ) -> Result<Vec<Result<SignatureBytes, CryptoError>>, CryptoError>;

    fn sign_one(self: &Self, message: &[u8], key_id: &KeyId) -> Result<[u8; 65], CryptoError>;

    /// Get the public key data for the key id provided.
    /// Fails whenever the key_id is not found in the keyring.
    fn get_public_key(self: &Self, key_id: &KeyId) -> Result<PublicKeyBytes, CryptoError>;
}

pub(crate) fn combine_sig_and_recovery(
    sig: SignatureBytesWithoutRecovery,
    recovery_term: u8,
) -> SignatureBytes {
    match (sig, recovery_term) {
        (
            [e1, e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12, e13, e14, e15, e16, e17, e18, e19, e20, e21, e22, e23, e24, e25, e26, e27, e28, e29, e30, e31, e32, e33, e34, e35, e36, e37, e38, e39, e40, e41, e42, e43, e44, e45, e46, e47, e48, e49, e50, e51, e52, e53, e54, e55, e56, e57, e58, e59, e60, e61, e62, e63, e64],
            rec,
        ) => [
            e1, e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12, e13, e14, e15, e16, e17, e18, e19,
            e20, e21, e22, e23, e24, e25, e26, e27, e28, e29, e30, e31, e32, e33, e34, e35, e36,
            e37, e38, e39, e40, e41, e42, e43, e44, e45, e46, e47, e48, e49, e50, e51, e52, e53,
            e54, e55, e56, e57, e58, e59, e60, e61, e62, e63, e64, rec,
        ],
    }
}

/// A helper function to sign a message in the style of ethereum
///
/// Reference implementation https://github.com/MaiaVictor/eth-lib/blob/d959c54faa1e1ac8d474028ed1568c5dce27cc7a/src/account.js#L55
/// This is called by web3.js https://github.com/ethereum/web3.js/blob/27c9679766bb4a965843e9bdaea575ea706202f1/packages/web3-eth-accounts/package.json#L18
pub fn eth_sign(message: &[u8], private_key: &SecretKey, prepend_preamble: bool) -> SignatureBytes {
    let hashed = eth_keccak_for_signature(message, prepend_preamble);
    // todo: there is something in this function that says "it is ok for the message to overflow.." that seems bad.
    let message = secp256k1::Message::parse(&hashed);
    let (sig, recovery) = secp256k1::sign(&message, &private_key);
    let recovery_term = recovery.serialize() + ETH_ADD_TO_V;
    let sig = sig.serialize();
    combine_sig_and_recovery(sig, recovery_term)
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
    pub fn get_keypair(self: &Self, key_id: &KeyId) -> Result<&EcdsaPair, CryptoError> {
        self.keys.get(&key_id.data).ok_or(CryptoError::KeyNotFound)
    }

    /// Get the private key associated with the key ID (for signing)
    pub fn get_private_key(self: &Self, key_id: &KeyId) -> Result<SecretKey, CryptoError> {
        let keypair = self.get_keypair(key_id)?;
        // note - it seems to me that this method is misnamed and this actually provides
        // the private key
        let private_key = keypair.seed();
        let private_key =
            secp256k1::SecretKey::parse(&private_key).map_err(|_| CryptoError::Unknown)?;

        Ok(private_key)
    }

    /// Get the eth address (bytes) associated with the given key id.
    pub fn get_eth_address(self: &Self, key_id: &KeyId) -> Result<AddressBytes, CryptoError> {
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
        messages: Vec<&[u8]>,
        key_id: &KeyId,
    ) -> Result<Vec<Result<SignatureBytes, CryptoError>>, CryptoError> {
        let private_key = self.get_private_key(key_id)?;

        let result = messages
            .iter()
            .map(|message| Ok(eth_sign(message, &private_key, false)))
            .collect();

        Ok(result)
    }

    fn sign_one(self: &Self, message: &[u8], key_id: &KeyId) -> Result<[u8; 65], CryptoError> {
        let private_key = self.get_private_key(key_id)?;
        Ok(eth_sign(message, &private_key, false))
    }

    /// Get the public key associated with the given key id.
    fn get_public_key(self: &Self, key_id: &KeyId) -> Result<PublicKeyBytes, CryptoError> {
        let private = self.get_private_key(key_id)?;
        // could not call serialize from the keypair so I had to re-derive the public key here
        let public = secp256k1::PublicKey::from_secret_key(&private);
        Ok(public_key_to_bytes(public))
    }
}

pub fn keyring() -> Box<dyn Keyring> {
    let keyring_type: Option<String> = std::env::var("KEYRING_TYPE").ok().into();
    let aws_kms = String::from("AWS_KMS");

    if keyring_type == Some(aws_kms) {
        Box::new(aws_kms::KmsKeyring::new())
    } else {
        Box::new(dev_keyring())
    }
}

pub(crate) const ETH_PRIVATE_KEY_ENV_VAR: &str = "ETH_KEY";

/// Get the recovery id and chain from the last byte of the signature
fn eth_get_chain(recovery_id: u8) -> Result<(u8, Option<u8>), CryptoError> {
    match recovery_id {
        0..=1 => Ok((recovery_id, None)),
        27..=28 => Ok((recovery_id - 27, None)),
        35..=255 => Ok((1 - recovery_id % 2, Some((recovery_id - 35) / 2))),
        2..=26 | 29..=34 => Err(CryptoError::RecoverError),
    }
}

/// Recovers the signer's address from the given signature and message. The message is _not_
/// expected to be a digest and is hashed inside.
pub fn eth_recover(
    message: &[u8],
    sig: &SignatureBytes,
    prepend_preamble: bool,
) -> Result<AddressBytes, CryptoError> {
    if sig.len() == 0 {
        return Err(CryptoError::RecoverError);
    }

    let last_byte_of_signature = sig[sig.len() - 1];
    let (recovery_id, _) = eth_get_chain(last_byte_of_signature)?;

    let recovery_id =
        secp256k1::RecoveryId::parse(recovery_id).map_err(|_| CryptoError::ParseError)?;

    let sig = secp256k1::Signature::parse_slice(&sig[..64]).map_err(|_| CryptoError::ParseError)?;
    let digested = eth_keccak_for_signature(&message, prepend_preamble);
    let message =
        secp256k1::Message::parse_slice(&digested).map_err(|_| CryptoError::ParseError)?;

    let recovered =
        secp256k1::recover(&message, &sig, &recovery_id).map_err(|_| CryptoError::RecoverError)?;
    let address = public_key_to_eth_address(recovered);

    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use our_std::convert::TryInto;
    use sp_core::Pair;

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
                signature: "0xa8037a6116c176a25e6fc224947fde9e79a2deaa0dd8b67b366fbdfdbffc01f953e41351267b20d4a89ebfe9c8f03c04de9b345add4a52f15bd026b63c8fb1501b".into(),
            }, TestCase {
                address: "0xEB014f8c8B418Db6b45774c326A0E64C78914dC0".into(),
                private_key: "0xbe6383dad004f233317e46ddb46ad31b16064d14447a95cc1d8c8d4bc61c3728".into(),
                data: "Some data!%$$%&@*".into(),
                signature: "0x05252412b097c5d080c994d1ea12abcee6f1cae23feb225517a0b691a66e12866b3f54292f9cfef98f390670b4d010fc4af7fcd46e41d72870602c117b14921c1c".into(),
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

    fn eth_decode_hex_address_unsafe(hex: String) -> AddressBytes {
        eth_decode_hex_unsafe(hex).try_into().unwrap()
    }
    fn eth_decode_hex_signature_unsafe(hex: String) -> SignatureBytes {
        eth_decode_hex_unsafe(hex).try_into().unwrap()
    }

    /// Test out eth signature function
    fn test_eth_sign_case(case: TestCase) {
        let private_key = eth_decode_hex_unsafe(case.private_key);
        let message: Vec<u8> = case.data.into();
        let secret_key = SecretKey::parse_slice(&private_key).unwrap();
        let actual_sig = eth_sign(&message, &secret_key, true);
        let expected_sig = eth_decode_hex_signature_unsafe(case.signature);
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
        let sig = eth_decode_hex_signature_unsafe(case.signature);
        let actual_address = eth_recover(&message, &sig, true).unwrap();
        let expected_address = eth_decode_hex_address_unsafe(case.address);
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
        let message: Vec<u8> = format!(
            "\x19Ethereum Signed Message:\n{}{}",
            case.data.len(),
            case.data
        )
        .as_bytes()
        .into();
        let messages: Vec<&[u8]> = vec![&message];
        // Note: we need to manually prepend here since `keyring` doesn't, itself
        // support prepending messages
        let sigs = keyring.sign(messages, &key_id).unwrap();
        assert_eq!(sigs.len(), 1);
        let actual_sig = sigs[0].as_ref().unwrap();
        let expected_sig = &eth_decode_hex_signature_unsafe(case.signature);
        assert_eq!(actual_sig, expected_sig);
    }

    #[test]
    fn test_sign() {
        get_test_cases().drain(..).for_each(test_sign_case);
    }

    fn test_public_key_case(case: TestCase) {
        let (key_id, keyring) = get_test_keyring_from_test_case(&case);
        let actual_address = keyring.get_eth_address(&key_id).unwrap();
        let expected_address = eth_decode_hex_address_unsafe(case.address);
        assert_eq!(actual_address, expected_address);
    }

    #[test]
    fn test_public_key() {
        get_test_cases().drain(..).for_each(test_public_key_case);
    }
}
