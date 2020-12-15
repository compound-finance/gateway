/// The crypto module for compound chain.
///
/// This entire crate is STD and designed to be exposed via
/// a runtime interface in the runtime-interfaces crate.

/// Should this be an associated type?
/// Should it use sp-std string?
/// Should there be a full-crypto version of this thing?
#[derive(Clone)]
pub struct KeyId {
    data: String,
}

/// The crypto error type
pub enum CryptoError {
    Unknown,
}

/// A keyring abstraction for HSMs
pub trait Keyring {
    /// Batch sign messages with the given key
    fn sign(messages: Vec<Vec<u8>>, key_id: &KeyId) -> Vec<Result<Vec<u8>, CryptoError>>;

    /// Get the public key data for the key id provided
    /// Fails whenever the key_id is not found in the keyring
    fn get_public_key(key_id: &KeyId) -> Result<Vec<u8>, CryptoError>;
}

/// Keyring
pub struct DevKeyring {}

impl Keyring for DevKeyring {
    fn sign(messages: Vec<Vec<u8>>, key_id: &KeyId) -> Vec<Result<Vec<u8>, CryptoError>> {
        unimplemented!()
    }

    fn get_public_key(key_id: &KeyId) -> Result<Vec<u8>, CryptoError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sign() {
        // foo
    }
}
