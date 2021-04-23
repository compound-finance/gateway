#[macro_use]
extern crate lazy_static;

use codec::{Decode, Encode};
use ethereum_client::EthereumBlock;
use gateway_crypto::CryptoError;
use sp_runtime_interface::pass_by::PassByCodec;
use std::{str, sync::Mutex};

#[derive(Clone, Decode, Encode, PassByCodec)]
pub struct Config {
    pub eth_starport_address: String,
    pub eth_starport_parent_block: EthereumBlock,
}

#[derive(Clone)]
pub struct ValidatorConfig {
    pub eth_key_id: String,
    pub eth_rpc_url: String,
    pub miner: String,
    pub opf_url: String,
}

/// XXX Possible sanity checks for config fields here
impl Config {
    pub fn update(&mut self, new: Config) {
        self.eth_starport_address = new.eth_starport_address;
    }
}

pub fn new_config(
    eth_starport_address: String,
    eth_starport_parent_block: EthereumBlock,
) -> Config {
    return Config {
        eth_starport_address,
        eth_starport_parent_block,
    };
}

pub type PriceFeedData = (Vec<(Vec<u8>, Vec<u8>)>, u64);

pub const NULL_ETH_BLOCK: EthereumBlock = EthereumBlock {
    hash: [0; 32],
    parent_hash: [0; 32],
    number: 0,
    events: vec![],
};

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(new_config("".into(), NULL_ETH_BLOCK));
    static ref VALIDATOR_CONFIG: Mutex<Option<ValidatorConfig>> = Mutex::new(None);
    static ref PRICE_FEED_DATA: Mutex<Option<PriceFeedData>> = Mutex::new(None);
}

pub fn initialize_validator_config(
    eth_key_id: Option<String>,
    eth_rpc_url: Option<String>,
    miner: Option<String>,
    opf_url: Option<String>,
) {
    match VALIDATOR_CONFIG.lock() {
        Ok(mut data_ref) => {
            *data_ref = Some(ValidatorConfig {
                eth_key_id: eth_key_id.unwrap_or(ETH_KEY_ID_DEFAULT.to_owned()),
                eth_rpc_url: eth_rpc_url.unwrap_or(ETH_RPC_URL_DEFAULT.to_owned()),
                miner: miner.unwrap_or(MINER_DEFAULT.to_owned()),
                opf_url: opf_url.unwrap_or(OPF_URL_DEFAULT.to_owned()),
            });
        }
        _ => (), // XXX todo: log?
    };
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

    /// Get the Ethereum Starport address.
    fn get_eth_starport_address() -> Option<String> {
        if let Ok(config) = CONFIG.lock() {
            if config.eth_starport_address.len() == 42 {
                return Some(config.eth_starport_address.clone());
            }
        }
        return None;
    }

    /// Get the Ethereum Starport parent block.
    fn get_eth_starport_parent_block() -> EthereumBlock {
        if let Ok(config) = CONFIG.lock() {
            return config.eth_starport_parent_block.clone();
        }
        return NULL_ETH_BLOCK;
    }
}

const ETH_KEY_ID_ENV_VAR: &str = "ETH_KEY_ID";
const ETH_RPC_URL_ENV_VAR: &str = "ETH_RPC_URL";
const MINER_ENV_VAR: &str = "MINER";
const OPF_URL_ENV_VAR: &str = "OPF_URL";
const ETH_FETCH_DEADLINE_ENV_VAR: &str = "ETH_FETCH_DEADLINE";

const ETH_KEY_ID_DEFAULT: &str = gateway_crypto::ETH_KEY_ID_ENV_VAR_DEV_DEFAULT;
const MINER_DEFAULT: &str = "Eth:0x0000000000000000000000000000000000000000";
const ETH_RPC_URL_DEFAULT: &str = "https://ropsten-eth.compound.finance";
const OPF_URL_DEFAULT: &str = "https://prices.compound.finance/coinbase";
/// Default value of ethereum client fetching deadline = 10s
const ETH_FETCH_DEADLINE_DEFAULT: u64 = 10000;

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
        // check env override
        if let Ok(eth_key_id_from_env) = std::env::var(ETH_KEY_ID_ENV_VAR) {
            if eth_key_id_from_env.len() > 0 {
                return Some(eth_key_id_from_env.into());
            }
        }
        // check config
        if let Ok(config) = VALIDATOR_CONFIG.lock() {
            if let Some(inner) = config.as_ref() {
                return Some(inner.eth_key_id.clone().into());
            }
        }
        // not set
        return Some(ETH_KEY_ID_DEFAULT.into());
    }

    /// Get the Ethereum node RPC URL
    fn get_eth_rpc_url() -> Option<String> {
        // check env override
        if let Ok(eth_rpc_url) = std::env::var(ETH_RPC_URL_ENV_VAR) {
            if eth_rpc_url.len() > 0 {
                return Some(eth_rpc_url);
            }
        }
        // check config
        if let Ok(config) = VALIDATOR_CONFIG.lock() {
            if let Some(inner) = config.as_ref() {
                return Some(inner.eth_rpc_url.clone());
            }
        }
        // not set
        return Some(ETH_RPC_URL_DEFAULT.into());
    }

    /// Get the open price feed URLs
    fn get_opf_url() -> Option<String> {
        // check env override
        if let Ok(opf_url) = std::env::var(OPF_URL_ENV_VAR) {
            if opf_url.len() > 0 {
                return Some(opf_url);
            }
        }
        // check config
        if let Ok(config) = VALIDATOR_CONFIG.lock() {
            if let Some(inner) = config.as_ref() {
                return Some(inner.opf_url.clone());
            }
        }
        // not set
        return Some(OPF_URL_DEFAULT.into());
    }

    /// Get the Miner address
    fn get_miner_address() -> Option<Vec<u8>> {
        // check env override
        if let Ok(miner) = std::env::var(MINER_ENV_VAR) {
            if miner.len() > 0 {
                return Some(miner.into());
            }
        }
        // check config
        if let Ok(config) = VALIDATOR_CONFIG.lock() {
            if let Some(inner) = config.as_ref() {
                return Some(inner.miner.clone().into());
            }
        }
        // not set
        return Some(MINER_DEFAULT.into());
    }

    /// Get ethereum client fetch events deadline in milliseconds
    fn get_eth_fetch_deadline() -> Option<u64> {
        // check env override
        if let Ok(deadline) = std::env::var(ETH_FETCH_DEADLINE_ENV_VAR) {
            return Some(deadline.parse::<u64>().unwrap());
        }
        // not set
        return Some(ETH_FETCH_DEADLINE_DEFAULT);
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
        let given_eth_starport_address = String::from("0xbbde1662bC3ED16aA8C618c9833c801F3543B587");
        let expected_eth_starport_address =
            String::from("0xbbde1662bC3ED16aA8C618c9833c801F3543B587");

        let config = new_config(given_eth_starport_address.clone(), NULL_ETH_BLOCK);
        // set in node
        config_interface::set(config);
        // ...later... in offchain worker context, get the configuration
        let actual_config = config_interface::get();
        let actual_eth_starport_address = actual_config.eth_starport_address;
        assert_eq!(expected_eth_starport_address, actual_eth_starport_address);
    }
}
