use compound_chain_runtime::{
    wasm_binary_unwrap, AccountId, BabeConfig, BalancesConfig, CashConfig, GenesisConfig,
    GrandpaConfig, Signature, SudoConfig, SystemConfig,
};
use sc_service::ChainType;
use sp_consensus_babe::AuthorityId as BabeId;
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

/// Generate various keys from seed
pub fn authority_keys_from_seed(seed: &str) -> (BabeId, GrandpaId) {
    (
        get_from_seed::<BabeId>(seed),
        get_from_seed::<GrandpaId>(seed),
    )
}

/// Get the properties key of the chain spec file - a basic valid configuration
fn get_properties() -> sc_service::Properties {
    let value = serde_json::json! ({
        "eth_rpc_url" : "https://goerli.infura.io/v3/975c0c48e2ca4649b7b332f310050e27"
        // todo: override with environment variable and/or cli param?
    });
    let as_object = value.as_object();
    let unwrapped = as_object.unwrap();
    unwrapped.clone()
}

fn development_genesis() -> GenesisConfig {
    testnet_genesis(
        // Initial PoA authorities
        vec![authority_keys_from_seed("Alice")],
        // Sudo account
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        // Pre-funded accounts
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
        ],
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
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        // Sudo account
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        // Pre-funded accounts
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
            get_account_id_from_seed::<sr25519::Public>("Eve"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie"),
            get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
            get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
            get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
        ],
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
        None,
        // Properties
        Some(get_properties()),
        // Extensions
        None,
    )
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
    initial_authorities: Vec<(BabeId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary_unwrap().to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(BalancesConfig {
            // Configure endowed accounts with initial balance of 1 << 60.
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 60))
                .collect(),
        }),
        pallet_babe: Some(BabeConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), 1))
                .collect(),
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        }),
        pallet_sudo: Some(SudoConfig {
            // Assign network admin rights.
            key: root_key,
        }),

        pallet_cash: Some(CashConfig {
            cash_balance: vec![], // XXX circular broken substrate -> hacked to gen GenesisConfig, but empty
        }),
    }
}

/// A helper function used to extract the runtime interface configuration used for offchain workers
/// from the properties attribute of the chain spec file.
pub fn extract_configuration_from_properties(
    properties: &sp_chain_spec::Properties,
) -> Option<runtime_interfaces::Config> {
    let key = "eth_rpc_url".to_owned();
    let eth_rpc_url = properties.get(&key)?;
    let eth_rpc_url_str = eth_rpc_url.as_str()?;
    // todo: eager validation of some kind here - basic sanity checking? or no?
    Some(runtime_interfaces::new_config(eth_rpc_url_str.into()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use compound_chain_runtime::BuildStorage;

    /// Best case scenario - we have the key we need in the properties map and we _can_ return
    /// the OCW configuration
    #[test]
    fn test_extract_configuration_from_properties_happy_path() {
        let expected = "hello world";
        let properties = serde_json::json!({ "eth_rpc_url": expected });
        let properties = properties.as_object().unwrap();

        let config = extract_configuration_from_properties(&properties);
        assert!(config.is_some());
        let config = config.unwrap();
        let actual = config.get_eth_rpc_url();
        // let actual = String::from_utf8(actual).unwrap();

        assert_eq!(actual.as_slice(), expected.as_bytes());
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
            vec![authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            vec![],
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
