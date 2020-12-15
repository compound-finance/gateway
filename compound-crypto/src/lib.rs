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
    /// request for each message in order.
    fn sign(messages: Vec<Vec<u8>>, key_id: &KeyId) -> Vec<Result<Vec<u8>, CryptoError>>;

    /// Get the public key data for the key id provided.
    /// Fails whenever the key_id is not found in the keyring.
    fn get_public_key(key_id: &KeyId) -> Result<Vec<u8>, CryptoError>;
}

/// Dev environment keyring
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
        // todo: write tests for dev setup
    }
}
