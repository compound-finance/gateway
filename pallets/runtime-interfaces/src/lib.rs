#[macro_use]
extern crate lazy_static;

use codec::{Decode, Encode};
use gateway_crypto::CryptoError;
use sp_runtime_interface::pass_by::PassByCodec;
use std::sync::Mutex;

#[derive(Clone, Decode, Encode, PassByCodec)]
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
        eth_starport_address,
    };
}

type PriceFeedData = (Vec<(Vec<u8>, Vec<u8>)>, u64);

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(new_config("".into()));
    static ref PRICE_FEED_DATA: Mutex<Option<PriceFeedData>> = Mutex::new(None);
}

/// The configuration interface for offchain workers. This is designed to manage configuration
/// that is specific to gateway AND distributed in the chain spec file. Ultimately
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
const ETH_KEY_ID_ENV_VAR_DEV_DEFAULT: &str = gateway_crypto::ETH_KEY_ID_ENV_VAR_DEV_DEFAULT;
const ETH_RPC_URL_ENV_VAR: &str = "ETH_RPC_URL";
const MINER_ENV_VAR: &str = "MINER";
const ETH_RPC_URL_ENV_VAR_DEV_DEFAULT: &str = "https://goerli-eth.compound.finance";
const OPF_URL_ENV_VAR: &str = "OPF_URL";
const OPF_URL_ENV_VAR_DEFAULT: &str = "https://prices.compound.finance/coinbase";

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

    /// Get the Miner address
    fn get_miner_address() -> Option<Vec<u8>> {
        std::env::var(MINER_ENV_VAR).ok().map(Into::into)
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
    ) -> Result<Vec<Result<gateway_crypto::SignatureBytes, CryptoError>>, CryptoError> {
        let keyring = gateway_crypto::keyring();
        let key_id = gateway_crypto::KeyId::from_utf8(key_id)?;
        let messages: Vec<&[u8]> = messages.iter().map(|e| e.as_slice()).collect();
        keyring.sign(messages, &key_id)
    }

    fn sign_one(message: Vec<u8>, key_id: Vec<u8>) -> Result<[u8; 65], CryptoError> {
        let keyring = gateway_crypto::keyring();
        let key_id = gateway_crypto::KeyId::from_utf8(key_id)?;
        keyring.sign_one(&message, &key_id)
    }

    fn get_public_key(key_id: Vec<u8>) -> Result<[u8; 64], CryptoError> {
        let keyring = gateway_crypto::keyring();
        let key_id = gateway_crypto::KeyId::from_utf8(key_id)?;
        keyring.get_public_key(&key_id)
    }

    /// Note - it is possible to run gateway_crypto in no-std / wasm environment but it is simply
    /// too slow to be feasible. We moved it out of gateway_crypto::no_std and now access eth_recover
    /// via this runtime interface.
    fn eth_recover(
        message: Vec<u8>,
        sig: gateway_crypto::SignatureBytes,
        prepend_preamble: bool,
    ) -> Result<gateway_crypto::AddressBytes, CryptoError> {
        gateway_crypto::eth_recover(&message, &sig, prepend_preamble)
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
    set_validator_config_dev_default(MINER_ENV_VAR, "");
}

#[sp_runtime_interface::runtime_interface]
pub trait PriceFeedInterface {
    fn get_price_data() -> Option<PriceFeedData> {
        match PRICE_FEED_DATA.lock() {
            Ok(inner) => inner.clone(),
            _ => None,
        }
    }

    fn get_price_data_ts() -> Option<u64> {
        match PRICE_FEED_DATA.lock() {
            Ok(inner) => inner.clone().map(|v| v.1),
            _ => None,
        }
    }

    fn set_price_data(prices: Vec<(Vec<u8>, Vec<u8>)>, timestamp: u64) {
        match PRICE_FEED_DATA.lock() {
            Ok(mut data_ref) => {
                *data_ref = Some((prices, timestamp));
            }
            _ => (),
        };
    }
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
