#[macro_use]
extern crate lazy_static;

use gateway_crypto::CryptoError;
use std::{str, sync::Mutex};

#[derive(Clone, Debug)]
pub struct ValidatorConfig {
    pub eth_key_id: String,
    pub eth_rpc_url: String,
    pub flow_server_url: String,
    pub miner: String,
    pub opf_url: String,
}

pub type PriceFeedData = (Vec<(Vec<u8>, Vec<u8>)>, u64);

lazy_static! {
    static ref VALIDATOR_CONFIG: Mutex<Option<ValidatorConfig>> = Mutex::new(None);
    static ref PRICE_FEED_DATA: Mutex<Option<PriceFeedData>> = Mutex::new(None);
}

pub fn initialize_validator_config(
    eth_key_id: Option<String>,
    eth_rpc_url: Option<String>,
    flow_server_url: Option<String>,
    miner: Option<String>,
    opf_url: Option<String>,
) {
    match VALIDATOR_CONFIG.lock() {
        Ok(mut data_ref) => {
            *data_ref = Some(ValidatorConfig {
                eth_key_id: eth_key_id.unwrap_or(ETH_KEY_ID_DEFAULT.to_owned()),
                eth_rpc_url: eth_rpc_url.unwrap_or(ETH_RPC_URL_DEFAULT.to_owned()),
                flow_server_url: flow_server_url.unwrap_or(FLOW_SERVER_URL_DEFAULT.to_owned()),
                miner: miner.unwrap_or(MINER_DEFAULT.to_owned()),
                opf_url: opf_url.unwrap_or(OPF_URL_DEFAULT.to_owned()),
            });
        }
        _ => (), // XXX todo: log?
    };
}

const ETH_KEY_ID_ENV_VAR: &str = "ETH_KEY_ID";
const ETH_RPC_URL_ENV_VAR: &str = "ETH_RPC_URL";
const FLOW_SERVER_URL_ENV_VAR: &str = "FLOW_SERVER_URL";
const MINER_ENV_VAR: &str = "MINER";
const OPF_URL_ENV_VAR: &str = "OPF_URL";

const ETH_KEY_ID_DEFAULT: &str = gateway_crypto::ETH_KEY_ID_ENV_VAR_DEV_DEFAULT;
const MINER_DEFAULT: &str = "Eth:0x0000000000000000000000000000000000000000";
const ETH_RPC_URL_DEFAULT: &str = "https://ropsten-eth.compound.finance";
const OPF_URL_DEFAULT: &str = "https://prices.compound.finance/coinbase";
const FLOW_SERVER_URL_DEFAULT: &str = "http://localhost:8089";

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

    /// Get the Flow events server URL
    fn get_flow_server_url() -> Option<String> {
        // check env override
        if let Ok(flow_server_url) = std::env::var(FLOW_SERVER_URL_ENV_VAR) {
            if flow_server_url.len() > 0 {
                return Some(flow_server_url);
            }
        }
        // check config
        if let Ok(config) = VALIDATOR_CONFIG.lock() {
            if let Some(inner) = config.as_ref() {
                return Some(inner.flow_server_url.clone());
            }
        }
        // not set
        return Some(FLOW_SERVER_URL_DEFAULT.into());
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
}
