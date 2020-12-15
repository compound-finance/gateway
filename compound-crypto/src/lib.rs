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
    /// Sign the message with the key
    /// I think we may need to make this async since there will be a lot of
    /// tings to sign, but perhaps use the KISS principle now and address any
    /// performance issues later as the crop up. Not sure about the latency of KMS
    /// or HSMs in general.
    /// Alternatively, it may be useful to _just_ add a batch signature function that says
    /// "sign all of these messages with the same key" and the implementation of that
    /// function may make use of async or existing batch functionality.
    /// In general, this function should do the hashing.
    fn sign(messages: Vec<Vec<u8>>, key_id: &KeyId) -> Vec<Result<Vec<u8>, CryptoError>>;

    /// Get the unencoded public key data for the key id provided
    /// Fails whenever the key_id is not found in the keyring
    fn get_public_key(key_id: &KeyId) -> Result<Vec<u8>, CryptoError>;
}

/// Keyring
pub struct DevKeyring {}

impl Keyring for DevKeyring {
    fn sign(message: Vec<Vec<u8>>, key_id: &KeyId) -> Vec<Result<Vec<u8>, CryptoError>> {
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
