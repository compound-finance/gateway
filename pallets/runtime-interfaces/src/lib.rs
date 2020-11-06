use sp_runtime_interface::pass_by::PassByCodec;

#[macro_use]
extern crate lazy_static;
use std::sync::Mutex;

#[derive(Clone, PassByCodec, codec::Encode, codec::Decode)]
pub struct Config {
    eth_rpc_url: Vec<u8>,
}

impl Config {
    pub fn update(&mut self, new: Config) {
        self.eth_rpc_url = new.eth_rpc_url
    }
}

pub fn new_config(eth_rpc_url: Vec<u8>) -> Config {
    return Config {
        eth_rpc_url: eth_rpc_url,
    };
}

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(new_config("".into()));
}

#[sp_runtime_interface::runtime_interface]
pub trait ConfigInterface {
    /// This method is designed to be used by the node as it starts up. The node reads
    /// the chain configuration from the "properties" key in the "chain spec" file. Those
    /// properties determine what goes into the Config object which will then be set here
    /// on startup.
    fn set(config: Config) {
        CONFIG.lock().unwrap().update(config);
    }

    /// This is desgined for use in the context of an offchain worker. The offchain worker may grab
    /// the configuration to determine where to make RPC calls among other parameters.
    fn get() -> Config {
        CONFIG.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        // this gets loaded from the json file "chain config file"
        let expected: Vec<u8> = "my eth rpc url".into();
        let config = new_config(expected.clone());
        // set in node
        config_interface::set(config);
        // ...later... in offchain worker context, get the configuration
        let actual_config = config_interface::get();
        let actual = actual_config.eth_rpc_url;
        assert_eq!(expected, actual);
    }
}
