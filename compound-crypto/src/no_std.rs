use codec::{Decode, Encode};

/// The crypto error type allows for various failure scenarios
///
/// * The key id provided is unknown
/// * The HSM is not available
/// * The HSM failed to sign this request for some other reason
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode)]
pub enum CryptoError {
    Unknown,
    KeyNotFound,
    KeyringLock,
    InvalidKeyId,
}

/// The default key id for the eth authority key (l1)
pub const ETH_KEY_ID_ENV_VAR_DEV_DEFAULT: &str = "my_eth_key_id";
