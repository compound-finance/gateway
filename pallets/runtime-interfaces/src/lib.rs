use sp_runtime_interface::pass_by::PassByCodec;

#[macro_use]
extern crate lazy_static;
use std::sync::Mutex;

#[derive(Clone, PassByCodec, codec::Encode, codec::Decode)]
pub struct Config {
    eth_rpc_url: Vec<u8>,
    eth_starport_address: Vec<u8>,
    eth_lock_event_topic: Vec<u8>,
}

/// XXX Possible sanity checks for config fields here
impl Config {
    pub fn update(&mut self, new: Config) {
        self.eth_rpc_url = new.eth_rpc_url;
        self.eth_starport_address = new.eth_starport_address;
        self.eth_lock_event_topic = new.eth_lock_event_topic;
    }

    pub fn get_eth_rpc_url(&self) -> Vec<u8> {
        self.eth_rpc_url.clone()
    }

    pub fn get_eth_starport_address(&self) -> Vec<u8> {
        self.eth_starport_address.clone()
    }

    pub fn get_eth_lock_event_topic(&self) -> Vec<u8> {
        self.eth_lock_event_topic.clone()
    }
}

pub fn new_config(
    eth_rpc_url: Vec<u8>,
    eth_starport_address: Vec<u8>,
    eth_lock_event_topic: Vec<u8>,
) -> Config {
    return Config {
        eth_rpc_url: eth_rpc_url,
        eth_starport_address: eth_starport_address,
        eth_lock_event_topic: eth_lock_event_topic,
    };
}

lazy_static! {
    static ref CONFIG: Mutex<Config> = Mutex::new(new_config("".into(), "".into(), "".into()));
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

    /// This is designed for use in the context of an offchain worker. The offchain worker may grab
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
        let given_eth_rpc_url: Vec<u8> = "my eth rpc url".into();
        let expected_eth_rpc_url: Vec<u8> = "my eth rpc url".into();

        let given_eth_starport_address: Vec<u8> =
            "0xbbde1662bC3ED16aA8C618c9833c801F3543B587".into();
        let expected_eth_starport_address: Vec<u8> =
            "0xbbde1662bC3ED16aA8C618c9833c801F3543B587".into();

        let given_eth_lock_event_topic: Vec<u8> =
            "0xec36c0364d931187a76cf66d7eee08fad0ec2e8b7458a8d8b26b36769d4d13f3".into();
        let expected_eth_lock_event_topic: Vec<u8> =
            "0xec36c0364d931187a76cf66d7eee08fad0ec2e8b7458a8d8b26b36769d4d13f3".into();

        let config = new_config(
            given_eth_rpc_url.clone(),
            given_eth_starport_address.clone(),
            given_eth_lock_event_topic.clone(),
        );
        // set in node
        config_interface::set(config);
        // ...later... in offchain worker context, get the configuration
        let actual_config = config_interface::get();
        let actual_eth_rpc_url = actual_config.eth_rpc_url;
        let actual_eth_starport_address = actual_config.eth_starport_address;
        let actual_eth_lock_event_topic = actual_config.eth_lock_event_topic;
        assert_eq!(expected_eth_rpc_url, actual_eth_rpc_url);
        assert_eq!(expected_eth_starport_address, actual_eth_starport_address);
        assert_eq!(expected_eth_lock_event_topic, actual_eth_lock_event_topic);
    }
}
