use gateway_runtime::{
    opaque, wasm_binary_unwrap, AccountId, AuraConfig, CashConfig, GenesisConfig, GrandpaConfig,
    OracleConfig, SessionConfig, Signature, SystemConfig,
};
use our_std::{convert::TryInto, str::FromStr};
use pallet_cash::{
    chains::{Chain, ChainBlock, ChainStarport, Ethereum},
    types::{AssetInfo, Timestamp, ValidatorKeys, APR},
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

fn development_genesis() -> GenesisConfig {
    testnet_genesis(
        // Initial PoA authorities
        vec![authority_keys_from_seed(
            "Alice",
            "0xc77494d805d2b455686ba6a6bdf1c68ecf6e1cd7",
        )],
        // Initial reporters
        vec![
            "0x85615b076615317c80f14cbad6501eec031cd51c",
            "0xfCEAdAFab14d46e20144F48824d0C09B1a03F2BC",
        ],
        // Initial assets
        vec![
            AssetInfo {
                liquidity_factor: FromStr::from_str("6789").unwrap(),
                ..AssetInfo::minimal(
                    FromStr::from_str("eth:0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE").unwrap(),
                    FromStr::from_str("ETH/18").unwrap(),
                )
            },
            AssetInfo {
                ticker: FromStr::from_str("USD").unwrap(),
                liquidity_factor: FromStr::from_str("6789").unwrap(),
                ..AssetInfo::minimal(
                    FromStr::from_str("eth:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap(),
                    FromStr::from_str("USDC/6").unwrap(),
                )
            },
        ],
        // Initial cash yield
        FromStr::from_str("0").unwrap(),
        // Initial timestamp
        wasm_timer::SystemTime::now()
            .duration_since(wasm_timer::UNIX_EPOCH)
            .expect("cannot get system time for genesis")
            .as_millis() as Timestamp,
        // Starports
        vec![],
        // Genesis Blocks
        vec![],
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
        None,
        // Extensions
        None,
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        // Initial PoA authorities
        vec![],
        // Initial reporters
        vec![],
        // Initial assets
        vec![],
        // Initial cash yield
        FromStr::from_str("0").unwrap(),
        // Initial timestamp
        0 as Timestamp,
        // Starports
        vec![],
        // Genesis Blocks
        vec![],
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
        None,
        // Extensions
        None,
    )
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
    initial_authorities: Vec<(AccountId, <Ethereum as Chain>::Address, AuraId, GrandpaId)>,
    reporters: Vec<&str>,
    assets: Vec<AssetInfo>,
    cash_yield: APR,
    last_yield_timestamp: Timestamp,
    starports: Vec<ChainStarport>,
    genesis_blocks: Vec<ChainBlock>,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary_unwrap().to_vec(),
            changes_trie_config: Default::default(),
        },
        pallet_aura: AuraConfig {
            authorities: vec![],
        },
        pallet_grandpa: GrandpaConfig {
            authorities: vec![],
        },

        pallet_session: SessionConfig {
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
        },

        pallet_cash: CashConfig {
            cash_yield,
            last_yield_timestamp,
            assets,
            // XXX initial authorities should just be Vec<ValidatorKeys>?
            validators: initial_authorities
                .iter()
                .map(|v| ValidatorKeys {
                    substrate_id: v.0.clone(),
                    eth_address: v.1,
                })
                .collect::<Vec<_>>(),
            starports: starports,
            genesis_blocks: genesis_blocks,
        },

        pallet_oracle: OracleConfig {
            reporters: reporters.try_into().unwrap(),
        },
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use gateway_runtime::BuildStorage;

    #[test]
    fn test_create_development_chain_spec() {
        development_config().build_storage().unwrap();
    }

    #[test]
    fn test_create_local_testnet_chain_spec() {
        integration_test_config_with_single_authority()
            .build_storage()
            .unwrap();
    }

    fn local_testnet_genesis_instant_single() -> GenesisConfig {
        testnet_genesis(
            // Initial PoA authorities
            vec![authority_keys_from_seed(
                "Alice",
                "0x435228f5ad6fc8ce7b4398456a72a2f14577d9cd",
            )],
            // Initial reporters
            vec![
                "0x85615b076615317c80f14cbad6501eec031cd51c",
                "0xfCEAdAFab14d46e20144F48824d0C09B1a03F2BC",
            ],
            // Initial assets
            vec![],
            // Initial cash yield
            FromStr::from_str("0").unwrap(),
            // Initial timestamp
            0 as Timestamp,
            // Starports
            vec![],
            // Genesis Blocks
            vec![],
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
