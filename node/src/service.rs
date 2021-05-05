//! Service implementation. Specialized wrapper over substrate service.

use crate::rpc;
use gateway_runtime::{self as node_runtime, opaque::Block, RuntimeApi};
use pallet_cash;
use pallet_oracle;
use sc_client_api::{ExecutorProvider, RemoteBackend};
use sc_executor::native_executor_instance;
use sc_service::{config::Configuration, error::Error as ServiceError, TaskManager};
use std::sync::Arc;
use std::time::Duration;

type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;

// A `CodeExecutor` which uses the native runtime when the wasm is equivalent to the natively compiled code.
native_executor_instance!(
    pub Executor,
    node_runtime::api::dispatch,
    node_runtime::native_version,
    (
        frame_benchmarking::benchmarking::HostFunctions,
        runtime_interfaces::validator_config_interface::HostFunctions,
        runtime_interfaces::keyring_interface::HostFunctions,
        runtime_interfaces::price_feed_interface::HostFunctions,
    ),
);

pub fn new_partial(
    config: &Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sp_consensus::DefaultImportQueue<Block, FullClient>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (
            sc_consensus_aura::AuraBlockImport<
                Block,
                FullClient,
                sc_finality_grandpa::GrandpaBlockImport<
                    FullBackend,
                    Block,
                    FullClient,
                    FullSelectChain,
                >,
                AuraPair,
            >,
            sc_finality_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
            impl Fn(rpc::DenyUnsafe, sc_rpc::SubscriptionTaskExecutor) -> rpc::IoHandler,
            sc_finality_grandpa::SharedVoterState,
        ),
    >,
    ServiceError,
> {
    let inherent_data_providers = sp_inherents::InherentDataProviders::new();
    inherent_data_providers
        .register_provider(pallet_cash::internal::miner::InherentDataProvider)
        .expect("Failed to register miner data provider");
    inherent_data_providers
        .register_provider(pallet_oracle::inherent::InherentDataProvider)
        .expect("Failed to register oracle data provider");

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
    let client = Arc::new(client);

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
    );

    let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
        client.clone(),
        &(client.clone() as Arc<_>),
        select_chain.clone(),
    )?;

    let aura_block_import = sc_consensus_aura::AuraBlockImport::<_, _, _, AuraPair>::new(
        grandpa_block_import.clone(),
        client.clone(),
    );

    let import_queue = sc_consensus_aura::import_queue::<_, _, _, AuraPair, _, _>(
        sc_consensus_aura::slot_duration(&*client)?,
        aura_block_import.clone(),
        Some(Box::new(grandpa_block_import.clone())),
        client.clone(),
        inherent_data_providers.clone(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
        sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
    )?;

    let (rpc_extensions_builder, shared_voter_state) = {
        let justification_stream = grandpa_link.justification_stream();
        let shared_authority_set = grandpa_link.shared_authority_set().clone();

        let finality_proof_provider = sc_finality_grandpa::FinalityProofProvider::new_for_service(
            backend.clone(),
            client.clone(),
        );

        let client = client.clone();
        let pool = transaction_pool.clone();
        let select_chain = select_chain.clone();
        let chain_spec = config.chain_spec.cloned_box();
        let shared_voter_state = sc_finality_grandpa::SharedVoterState::empty();
        let shared_voter_state_copy = shared_voter_state.clone();

        let rpc_extensions_builder = move |deny_unsafe, subscription_executor| {
            let deps = rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                select_chain: select_chain.clone(),
                chain_spec: chain_spec.cloned_box(),
                deny_unsafe,
                grandpa: rpc::GrandpaDeps {
                    shared_voter_state: shared_voter_state.clone(),
                    shared_authority_set: shared_authority_set.clone(),
                    justification_stream: justification_stream.clone(),
                    subscription_executor,
                    finality_provider: finality_proof_provider.clone(),
                },
            };

            rpc::create_full(deps)
        };

        (rpc_extensions_builder, shared_voter_state_copy)
    };

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain: select_chain.clone(),
        transaction_pool,
        inherent_data_providers,
        other: (
            aura_block_import,
            grandpa_link,
            rpc_extensions_builder,
            shared_voter_state,
        ),
    })
}

pub fn new_full(mut config: Configuration) -> Result<TaskManager, ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        inherent_data_providers,
        other: (aura_block_import, grandpa_link, rpc_extensions_builder, shared_voter_state),
    } = new_partial(&config)?;

    config
        .network
        .notifications_protocols
        .push(sc_finality_grandpa::GRANDPA_PROTOCOL_NAME.into());

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: None,
            block_announce_validator_builder: None,
        })?;

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &config,
            backend.clone(),
            task_manager.spawn_handle(),
            client.clone(),
            network.clone(),
        );
    }

    let role = config.role.clone();
    let force_authoring = config.force_authoring;
    let backoff_authoring_blocks =
        Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
    let name = config.network.node_name.clone();
    let enable_grandpa = !config.disable_grandpa;
    let prometheus_registry = config.prometheus_registry().cloned();
    let telemetry_connection_sinks = sc_service::TelemetryConnectionSinks::default();

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config,
        backend: backend.clone(),
        network: network.clone(),
        client: client.clone(),
        keystore: keystore_container.sync_keystore(),
        rpc_extensions_builder: Box::new(rpc_extensions_builder),
        transaction_pool: transaction_pool.clone().clone(),
        task_manager: &mut task_manager,
        on_demand: None,
        remote_blockchain: None,
        telemetry_connection_sinks: telemetry_connection_sinks.clone(),
        network_status_sinks,
        system_rpc_tx,
    })?;

    if role.is_authority() {
        let proposer = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
        );

        let can_author_with =
            sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let aura = sc_consensus_aura::start_aura::<_, _, _, _, _, AuraPair, _, _, _, _>(
            sc_consensus_aura::slot_duration(&*client)?,
            client,
            select_chain,
            aura_block_import,
            proposer,
            network.clone(),
            inherent_data_providers.clone(),
            force_authoring,
            backoff_authoring_blocks,
            keystore_container.sync_keystore(),
            can_author_with,
        )?;

        task_manager
            .spawn_essential_handle()
            .spawn_blocking("aura", aura);
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(keystore_container.sync_keystore())
    } else {
        None
    };

    if enable_grandpa {
        let grandpa_config = sc_finality_grandpa::Config {
            // FIXME #1578 make this available through chainspec
            gossip_duration: Duration::from_millis(333),
            justification_period: 512,
            name: Some(name),
            observer_enabled: false,
            keystore,
            is_authority: role.is_authority(),
        };

        // start the full GRANDPA voter
        // NOTE: non-authorities could run the GRANDPA observer protocol, but at
        // this point the full voter should provide better guarantees of block
        // and vote data availability than the observer. The observer has not
        // been tested extensively yet and having most nodes in a network run it
        // could lead to finality stalls.
        let grandpa_params = sc_finality_grandpa::GrandpaParams {
            config: grandpa_config,
            link: grandpa_link,
            network: network,
            telemetry_on_connect: Some(telemetry_connection_sinks.on_connect_stream()),
            voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry,
            shared_voter_state: shared_voter_state,
        };

        task_manager.spawn_essential_handle().spawn_blocking(
            "grandpa-voter",
            sc_finality_grandpa::run_grandpa_voter(grandpa_params)?,
        );
    }

    network_starter.start_network();

    Ok(task_manager)
}

pub fn new_light(mut config: Configuration) -> Result<TaskManager, ServiceError> {
    let (client, backend, keystore_container, mut task_manager, on_demand) =
        sc_service::new_light_parts::<Block, RuntimeApi, Executor>(&config)?;

    config
        .network
        .notifications_protocols
        .push(sc_finality_grandpa::GRANDPA_PROTOCOL_NAME.into());

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
        on_demand.clone(),
    ));

    let (grandpa_block_import, _) = sc_finality_grandpa::block_import(
        client.clone(),
        &(client.clone() as Arc<_>),
        select_chain.clone(),
    )?;

    let aura_block_import = sc_consensus_aura::AuraBlockImport::<_, _, _, AuraPair>::new(
        grandpa_block_import.clone(),
        client.clone(),
    );

    let import_queue = sc_consensus_aura::import_queue::<_, _, _, AuraPair, _, _>(
        sc_consensus_aura::slot_duration(&*client)?,
        aura_block_import,
        Some(Box::new(grandpa_block_import)),
        client.clone(),
        sp_inherents::InherentDataProviders::new(),
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
        sp_consensus::NeverCanAuthor,
    )?;

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: Some(on_demand.clone()),
            block_announce_validator_builder: None,
        })?;
    network_starter.start_network();

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &config,
            backend.clone(),
            task_manager.spawn_handle(),
            client.clone(),
            network.clone(),
        );
    }

    let light_deps = rpc::LightDeps {
        remote_blockchain: backend.remote_blockchain(),
        fetcher: on_demand.clone(),
        client: client.clone(),
        pool: transaction_pool.clone(),
    };

    let rpc_extensions = rpc::create_light(light_deps);

    let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        remote_blockchain: Some(backend.remote_blockchain()),
        rpc_extensions_builder: Box::new(sc_service::NoopRpcExtensionBuilder(rpc_extensions)),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        on_demand: Some(on_demand),
        telemetry_connection_sinks: sc_service::TelemetryConnectionSinks::default(),
        config,
        client: client.clone(),
        keystore: keystore_container.sync_keystore(),
        backend,
        network: network.clone(),
        network_status_sinks,
        system_rpc_tx,
    })?;

    Ok(task_manager)
}
