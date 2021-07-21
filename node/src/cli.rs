use sc_cli::RunCmd;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct GatewayCmd {
    /// Set the ETH Key ID for AWS KMS integration
    #[structopt(long = "eth-key-id")]
    pub eth_key_id: Option<String>,

    /// Set the ETH RPC Url for interfacing with ethereum
    #[structopt(long = "eth-rpc-url")]
    pub eth_rpc_url: Option<String>,

    /// Set the Flow Server Url for interfacing with Flow fetch server
    #[structopt(long = "flow-server-url")]
    pub flow_server_url: Option<String>,

    /// Set the miner address (only useful for validator)
    #[structopt(long = "miner")]
    pub miner: Option<String>,

    /// Open price feed URL
    #[structopt(long = "opf-url")]
    pub opf_url: Option<String>,
}

#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[structopt(flatten)]
    pub run: RunCmd,

    #[structopt(flatten)]
    pub gateway: GatewayCmd,
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    /// Build a chain specification.
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Validate blocks.
    CheckBlock(sc_cli::CheckBlockCmd),

    /// Export blocks.
    ExportBlocks(sc_cli::ExportBlocksCmd),

    /// Export the state of a given block into a chain spec.
    ExportState(sc_cli::ExportStateCmd),

    /// Import blocks.
    ImportBlocks(sc_cli::ImportBlocksCmd),

    /// Remove the whole chain.
    PurgeChain(sc_cli::PurgeChainCmd),

    /// Revert the chain to a previous state.
    Revert(sc_cli::RevertCmd),

    /// The custom benchmark subcommmand benchmarking runtime pallets.
    #[cfg(feature = "runtime-benchmarks")]
    #[structopt(name = "benchmark", about = "Benchmark runtime pallets.")]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),
}
