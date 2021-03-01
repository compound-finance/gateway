use compound_chain_runtime::{
    opaque, wasm_binary_unwrap, AccountId, AuraConfig, CashConfig, GenesisConfig, GrandpaConfig,
    SessionConfig, Signature, SudoConfig, SystemConfig,
};
use our_std::{convert::TryInto, str::FromStr};
use pallet_cash::{
    chains::{Chain, Ethereum},
    types::{AssetInfo, Timestamp, ValidatorKeys},
};

use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

// XXX are we going to setup our own telemetry server?
// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

// XXX just construct ValidatorKeys here and should include aura/grandpa?
/// Generate various keys from seed
pub fn authority_keys_from_seed(
    seed: &str,
    identity: &str,
) -> (AccountId, <Ethereum as Chain>::Address, AuraId, GrandpaId) {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        <Ethereum as Chain>::str_to_address(identity).unwrap(),
        get_from_seed::<AuraId>(seed),
        get_from_seed::<GrandpaId>(seed),
    )
}

/// Get the properties key of the chain spec file - a basic valid configuration
fn get_properties() -> sc_service::Properties {
    let value = serde_json::json! ({
        "eth_starport_address" : "0x5dCE7D554f5237F67bb2C6708e45291B3c6e0389"
    });
    let as_object = value.as_object();
    let unwrapped = as_object.unwrap();
    unwrapped.clone()
}

fn development_genesis() -> GenesisConfig {
    testnet_genesis(
        // Initial PoA authorities
        vec![authority_keys_from_seed(
            "Alice",
            "0xc77494d805d2b455686ba6a6bdf1c68ecf6e1cd7",
        )],
        // Sudo account
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        true,
    )
}

pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        // Name
        "Development",
        // ID
        "dev",
        ChainType::Development,
        development_genesis,
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        Some(get_properties()),
        // Extensions
        None,
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        // Initial PoA authorities
        vec![
            authority_keys_from_seed("Alice", "0xc77494d805d2b455686ba6a6bdf1c68ecf6e1cd7"),
            authority_keys_from_seed("Bob", "0x435228f5ad6fc8ce7b4398456a72a2f14577d9cd"),
        ],
        // Sudo account
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        true,
    )
}

pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        // Name
        "Local Testnet",
        // ID
        "local_testnet",
        ChainType::Local,
        local_testnet_genesis,
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        Some("local"),
        // Properties
        Some(get_properties()),
        // Extensions
        None,
    )
}

fn staging_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        // Initial PoA authorities
        vec![
            authority_keys_from_seed("Alice", "0xc77494d805d2b455686ba6a6bdf1c68ecf6e1cd7"),
            // authority_keys_from_seed("Bob", "0x435228f5ad6fc8ce7b4398456a72a2f14577d9cd"),
            // authority_keys_from_seed("Charlie", "0x435228f5ad6fc8ce7b4398456a72a2f14577d9cd"),
        ],
        // Sudo account
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        true,
    )
}

pub fn staging_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        // Name
        "Staging Testnet",
        // ID
        "staging_testnet",
        ChainType::Live,
        staging_testnet_genesis,
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        Some("compound-chain-staging"),
        // Properties
        Some(get_properties()),
        // Extensions
        None,
    )
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
    initial_authorities: Vec<(AccountId, <Ethereum as Chain>::Address, AuraId, GrandpaId)>,
    root_key: AccountId,
    _enable_println: bool,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary_unwrap().to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_aura: Some(AuraConfig {
            authorities: vec![],
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: vec![],
        }),
        pallet_sudo: Some(SudoConfig {
            // Assign network admin rights.
            key: root_key,
        }),

        pallet_session: Some(SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        opaque::SessionKeys {
                            aura: x.2.clone(),
                            grandpa: x.3.clone(),
                        },
                    )
                })
                .collect::<Vec<_>>(),
        }),

        pallet_cash: Some(CashConfig {
            cash_yield: FromStr::from_str("0").unwrap(),
            last_yield_timestamp: wasm_timer::SystemTime::now()
                .duration_since(wasm_timer::UNIX_EPOCH)
                .expect("cannot get system time for genesis")
                .as_millis() as Timestamp,

            assets: vec![
                AssetInfo {
                    liquidity_factor: FromStr::from_str("6789").unwrap(),
                    ..AssetInfo::minimal(
                        FromStr::from_str("eth:0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE")
                            .unwrap(),
                        FromStr::from_str("ETH/18").unwrap(),
                    )
                },
                AssetInfo {
                    ticker: FromStr::from_str("USD").unwrap(),
                    liquidity_factor: FromStr::from_str("6789").unwrap(),
                    ..AssetInfo::minimal(
                        FromStr::from_str("eth:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")
                            .unwrap(),
                        FromStr::from_str("USDC/6").unwrap(),
                    )
                },
            ],

            reporters: vec![
                "0x85615b076615317c80f14cbad6501eec031cd51c",
                "0xfCEAdAFab14d46e20144F48824d0C09B1a03F2BC",
            ]
            .try_into()
            .unwrap(),

            // XXX initial authorities should just be Vec<ValidatorKeys>?
            validators: initial_authorities
                .iter()
                .map(|v| ValidatorKeys {
                    substrate_id: v.0.clone(),
                    eth_address: v.1,
                })
                .collect::<Vec<_>>(),
        }),
    }
}

/// A helper function used to extract the runtime interface configuration used for offchain workers
/// from the properties attribute of the chain spec file.
pub fn extract_configuration_from_properties(
    properties: &sp_chain_spec::Properties,
) -> Option<runtime_interfaces::Config> {
    let key_address = "eth_starport_address".to_owned();
    let eth_starport_address = properties.get(&key_address)?;
    let eth_starport_address_str = eth_starport_address.as_str()?;

    // todo: eager validation of some kind here - basic sanity checking? or no?
    Some(runtime_interfaces::new_config(
        eth_starport_address_str.into(),
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use compound_chain_runtime::BuildStorage;

    /// Best case scenario - we have the key we need in the properties map and we _can_ return
    /// the OCW configuration
    #[test]
    fn test_extract_configuration_from_properties_happy_path() {
        let expected_starport = "hello starport";
        let expected_topic = "hello topic";
        let properties = serde_json::json!({ "eth_starport_address": expected_starport });
        let properties = properties.as_object().unwrap();

        let config = extract_configuration_from_properties(&properties);
        assert!(config.is_some());
        let config = config.unwrap();
        let actual_eth_starport_address = config.get_eth_starport_address();
        // let actual = String::from_utf8(actual).unwrap();

        assert_eq!(
            actual_eth_starport_address.as_slice(),
            expected_starport.as_bytes()
        );
    }

    /// Bad case - we do _not_ have the keys we need to return the OCW configuration
    #[test]
    fn test_extract_configuration_from_properties_missing_keys() {
        let properties = serde_json::json!({ "wrong_key": "some value" });
        let properties = properties.as_object().unwrap();

        let config = extract_configuration_from_properties(&properties);
        assert!(config.is_none());
    }

    /// Bad case - we do have the keys we need but we have the wrong data types
    #[test]
    fn test_extract_configuration_from_properties_wrong_type() {
        let properties = serde_json::json!({ "eth_rpc_url": 0 });
        let properties = properties.as_object().unwrap();

        let config = extract_configuration_from_properties(&properties);
        assert!(config.is_none());
    }

    #[test]
    fn test_create_development_chain_spec() {
        development_config().build_storage().unwrap();
    }

    #[test]
    fn test_create_local_testnet_chain_spec() {
        local_testnet_config().build_storage().unwrap();
    }

    fn local_testnet_genesis_instant_single() -> GenesisConfig {
        testnet_genesis(
            vec![authority_keys_from_seed(
                "Alice",
                "0x435228f5ad6fc8ce7b4398456a72a2f14577d9cd",
            )],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            false,
        )
    }

    /// Local testnet config (single validator - Alice)
    pub fn integration_test_config_with_single_authority() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis_instant_single,
            vec![],
            None,
            None,
            None,
            Default::default(),
        )
    }

    /// Local testnet config (multivalidator Alice + Bob)
    pub fn integration_test_config_with_two_authorities() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis,
            vec![],
            None,
            None,
            None,
            Default::default(),
        )
    }
}
