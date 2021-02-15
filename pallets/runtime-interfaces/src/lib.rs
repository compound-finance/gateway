use sp_runtime_interface::pass_by::PassByCodec;

#[macro_use]
extern crate lazy_static;
use compound_crypto::CryptoError;
use std::sync::Mutex;

#[derive(Clone, PassByCodec, codec::Encode, codec::Decode)]
pub struct Config {
    eth_starport_address: Vec<u8>,
}

/// XXX Possible sanity checks for config fields here
impl Config {
    pub fn update(&mut self, new: Config) {
        self.eth_starport_address = new.eth_starport_address;
    }

    pub fn get_eth_starport_address(&self) -> Vec<u8> {
        self.eth_starport_address.clone()
    }
}

pub fn new_config(eth_starport_address: Vec<u8>) -> Config {
    return Config {
        eth_starport_address: eth_starport_address,
    };
}

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(new_config("".into()));
}

/// The configuration interface for offchain workers. This is designed to manage configuration
/// that is specific to compound chain AND distributed in the chain spec file. Ultimately
/// the things that are being configured here should not be changed by any honest validators.
/// Each chain spec file will have different values for these configurations by design, for example
/// testnet may point to an Ethereum testnet starport address and lock event topic.
#[sp_runtime_interface::runtime_interface]
pub trait ConfigInterface {
    /// This method is designed to be used by the node as it starts up. The node reads
    /// the chain configuration from the "properties" key in the "chain spec" file. Those
    /// properties determine what goes into the Config object which will then be set here
    /// on startup.
    fn set(config: Config) {
        CONFIG.lock().unwrap().update(config);
    }

    /// This is designed for use in the context of an offchain worker. The offchain worker may grab
    /// the configuration to determine where to make RPC calls among other parameters.
    fn get() -> Config {
        CONFIG.lock().unwrap().clone()
    }
}

const ETH_KEY_ID_ENV_VAR: &str = "ETH_KEY_ID";
const ETH_KEY_ID_ENV_VAR_DEV_DEFAULT: &str = compound_crypto::ETH_KEY_ID_ENV_VAR_DEV_DEFAULT;
const ETH_RPC_URL_ENV_VAR: &str = "ETH_RPC_URL";
const ETH_RPC_URL_ENV_VAR_DEV_DEFAULT: &str =
    "https://goerli.infura.io/v3/975c0c48e2ca4649b7b332f310050e27";
const OPF_URL_ENV_VAR: &str = "OPF_URL";
const OPF_URL_ENV_VAR_DEFAULT: &str = "";

/// The ValidatorConfigInterface is designed to be modified as needed by the validators. This means
/// that each validator should be modifying the values here. For example, the ETH_KEY_ID is set
/// by each validator separately corresponding to their HSM configuration and key ID that they
/// have set up on the backend. This has nothing to do with the "chain spec" file and thus is
/// a separate interface to avoid issues with validators having to modify the chain spec file
/// perhaps becoming byzantine in the process via human error.
///
/// It is also not a file based configuration but an environment variable based configuration.
/// This allows for more straightforward configuration in some cases and separates values that
/// are integral to the system from validator specific values that otherwise would be intermingled.
#[sp_runtime_interface::runtime_interface]
pub trait ValidatorConfigInterface {
    /// Get the Key ID for the Ethereum key.
    ///
    /// Downstream this is used to feed the keyring for signing and obtaining the corresponding
    /// public key.
    fn get_eth_key_id() -> Option<Vec<u8>> {
        std::env::var(ETH_KEY_ID_ENV_VAR).ok().map(Into::into)
    }

    /// Get the Ethereum node RPC URL
    fn get_eth_rpc_url() -> Option<Vec<u8>> {
        std::env::var(ETH_RPC_URL_ENV_VAR).ok().map(Into::into)
    }

    /// Get the open price feed URLs
    fn get_opf_url() -> Option<Vec<u8>> {
        std::env::var(OPF_URL_ENV_VAR).ok().map(Into::into)
    }
}

#[sp_runtime_interface::runtime_interface]
pub trait KeyringInterface {
    /// Get the Key ID for the Ethereum key.
    ///
    /// Downstream this is used to feed the keyring for signing and obtaining the corresponding
    /// public key.
    fn sign(
        messages: Vec<Vec<u8>>,
        key_id: Vec<u8>,
    ) -> Result<Vec<Result<compound_crypto::SignatureBytes, CryptoError>>, CryptoError> {
        let keyring = compound_crypto::KEYRING
            .lock()
            .map_err(|_| CryptoError::KeyringLock)?;
        let key_id = compound_crypto::KeyId::from_utf8(key_id)?;
        let messages: Vec<&[u8]> = messages.iter().map(|e| e.as_slice()).collect();
        keyring.sign(messages, &key_id)
    }

    fn sign_one(message: Vec<u8>, key_id: Vec<u8>) -> Result<[u8; 65], CryptoError> {
        let keyring = compound_crypto::KEYRING
            .lock()
            .map_err(|_| CryptoError::KeyringLock)?;
        let key_id = compound_crypto::KeyId::from_utf8(key_id)?;
        keyring.sign_one(&message, &key_id)
    }

    fn get_public_key(key_id: Vec<u8>) -> Result<[u8; 64], CryptoError> {
        let keyring = compound_crypto::KEYRING
            .lock()
            .map_err(|_| CryptoError::KeyringLock)?;
        let key_id = compound_crypto::KeyId::from_utf8(key_id)?;
        keyring.get_public_key(&key_id)
    }
}

/// Set an environment variable to a value if it is not already set to an existing value.
fn set_validator_config_dev_default(key: &str, value: &str) {
    let existing = std::env::var(key);
    if existing.is_ok() {
        return;
    }

    std::env::set_var(key, value);
}

/// For the local dev environment we want some sane defaults for these configuration values
/// without always having to set up the developer's machine, with, say, a `.env` file
/// or a run shell script. We would like to just stick with `cargo run -- ...`. To that end
/// when the `--dev` flag is passed, these defaults are used but only when an existing environment
/// variable is not already set.
pub fn set_validator_config_dev_defaults() {
    set_validator_config_dev_default(ETH_KEY_ID_ENV_VAR, ETH_KEY_ID_ENV_VAR_DEV_DEFAULT);
    set_validator_config_dev_default(ETH_RPC_URL_ENV_VAR, ETH_RPC_URL_ENV_VAR_DEV_DEFAULT);
    set_validator_config_dev_default(OPF_URL_ENV_VAR, OPF_URL_ENV_VAR_DEFAULT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        // this gets loaded from the json file "chain config file"
        let given_eth_starport_address: Vec<u8> =
            "0xbbde1662bC3ED16aA8C618c9833c801F3543B587".into();
        let expected_eth_starport_address: Vec<u8> =
            "0xbbde1662bC3ED16aA8C618c9833c801F3543B587".into();

        let config = new_config(given_eth_starport_address.clone());
        // set in node
        config_interface::set(config);
        // ...later... in offchain worker context, get the configuration
        let actual_config = config_interface::get();
        let actual_eth_starport_address = actual_config.eth_starport_address;
        assert_eq!(expected_eth_starport_address, actual_eth_starport_address);
    }
}
