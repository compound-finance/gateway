use sc_cli::RunCmd;
use std::collections::HashMap;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct GatewayCmd {
    #[structopt(short = "e", long = "env", help = "set validator specific settings")]
    /// The currently available options are
    ///
    /// ETH_KEY_ID
    /// ETH_RPC_URL
    /// MINER
    /// OPF_URL
    ///
    /// example ./gateway .... --env ETH_RPC_URL=http://... ETH_KEY_ID=.. MINER=Eth:0x01234567890123456789 OPF_URL=http://....
    pub env: Vec<String>,
}

impl GatewayCmd {
    /// Convert the CLI mapping given into a
    pub fn parse_cli_mapping(self) -> HashMap<String, String> {
        let mut return_value = HashMap::with_capacity(self.env.len());
        for e in self.env {
            if let Some(i) = e.find("=") {
                let (key, value) = e.split_at(i);
                // value still has `=` in front
                if key.len() == 0 {
                    panic!("Empty validator configuration key detected");
                }
                if value.len() == 1 {
                    panic!("Empty value for key {}", key);
                }
                // strip the = from the front of `value`
                return_value.insert(key.to_string(), value[1..].to_string());
            } else {
                panic!("The value `{}` supplied as a validator configuration should be a key and value separated by `=`.", e);
            }
        }

        return_value
    }
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

#[cfg(test)]
pub mod test {
    use super::GatewayCmd;

    #[test]
    fn test_parse_cli_mapping() {
        let args: Vec<String> = Vec::from([
            "ETH_RPC_URL=expected eth rpc url".into(),
            "MINER=expected miner".into(),
            "ETH_KEY_ID=expected eth key id".into(),
            "OPF_URL=expected opf url".into(),
        ]);

        let unit_under_test = GatewayCmd { env: args };

        let mut actual = unit_under_test.parse_cli_mapping();

        assert_eq!(
            actual.remove("ETH_RPC_URL"),
            Some("expected eth rpc url".into())
        );
        assert_eq!(actual.remove("MINER"), Some("expected miner".into()));
        assert_eq!(
            actual.remove("ETH_KEY_ID"),
            Some("expected eth key id".into())
        );
        assert_eq!(actual.remove("OPF_URL"), Some("expected opf url".into()));
    }
}
