use crate::{
    eth_decode_hex, CryptoError, InMemoryKeyring, KeyId, ETH_KEY_ID_ENV_VAR_DEV_DEFAULT,
    ETH_PRIVATE_KEY_ENV_VAR,
};
use sp_core::ecdsa::Pair as EcdsaPair;
use sp_core::Pair;

pub(crate) const ETH_PRIVATE_KEY_DEFAULT_VALUE: &str =
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

const ETH_KEY_ID_ENV_VAR: &str = "ETH_KEY_ID";

/// Sets up the development keyring, an in memory keyring loaded with the default key
/// for signing messages headed to ethereum.
///
/// WARNING - This function will panic whenever a bad private key is set in the environment
/// variable. That is "ok" because it should only be used during boot.
pub fn dev_keyring() -> InMemoryKeyring {
    let mut keyring = InMemoryKeyring::new();
    let eth_key_id: KeyId = if let Ok(eth_key_id_from_env) = std::env::var(ETH_KEY_ID_ENV_VAR) {
        if eth_key_id_from_env.len() > 0 {
            KeyId::from(eth_key_id_from_env)
        } else {
            ETH_KEY_ID_ENV_VAR_DEV_DEFAULT.into()
        }
    } else {
        ETH_KEY_ID_ENV_VAR_DEV_DEFAULT.into()
    };

    let pair = match get_eth_private_key_from_environment_variable() {
        Ok(pair) => pair,
        Err(CryptoError::EnvironmentVariablePrivateKeyNotSet) => {
            // fall back to the default value used in unit tests and used on the testnet
            let default_private_key_hex =
                hex::decode(crate::dev::ETH_PRIVATE_KEY_DEFAULT_VALUE).unwrap();
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
