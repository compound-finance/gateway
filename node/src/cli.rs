use sc_cli::RunCmd;
use std::collections::HashMap;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct GatewayCmd {
    // collect args after -- like cargo, eg cargo run -- --help for generic key-value configuration
    // expected format is --eth-key-id abcdefg --miner 123456789
    // OR
    // ETH_KEY_ID abcdefg MINER 123456789
    // equals = linking not supported as in --eth-key-id=abcdefg
    #[structopt()]
    pub args: Vec<String>,
}

impl GatewayCmd {
    fn kebab_with_leading_dashes_to_screaming_snake(s: &str) -> String {
        let range = if s.starts_with("--") { 2.. } else { 0.. };
        s[range].replace("-", "_").to_uppercase()
    }

    pub fn parse_cli_mapping(self) -> HashMap<String, String> {
        if self.args.len() % 2 != 0 {
            // note it is ok to panic here because this is a critical part of the booting up
            // process of the node and is not expected to result in a bad state
            panic!("odd number of arguments provided to gateway key value pair configuration.")
        }

        let mut return_value = HashMap::with_capacity(self.args.len() / 2);

        for i in (0..self.args.len()).step_by(2) {
            let raw_key = &self.args[i];
            // note - we don't worry about going out of bounds because we know we
            // have an even number of elements from above
            let value = &self.args[i + 1];

            let key = GatewayCmd::kebab_with_leading_dashes_to_screaming_snake(raw_key);

            return_value.insert(key, value.clone());
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
    fn test_kebab_with_leading_dashes_to_screaming_snake() {
        assert_eq!(
            GatewayCmd::kebab_with_leading_dashes_to_screaming_snake("--abc-def-ghi"),
            "ABC_DEF_GHI".to_string()
        );
        assert_eq!(
            GatewayCmd::kebab_with_leading_dashes_to_screaming_snake("abc-def-ghi"),
            "ABC_DEF_GHI".to_string()
        );
        assert_eq!(
            GatewayCmd::kebab_with_leading_dashes_to_screaming_snake("--abc--def-ghi"),
            "ABC__DEF_GHI".to_string()
        );
        assert_eq!(
            GatewayCmd::kebab_with_leading_dashes_to_screaming_snake("ABC_DEF_GHI"),
            "ABC_DEF_GHI".to_string()
        );
        assert_eq!(
            GatewayCmd::kebab_with_leading_dashes_to_screaming_snake("--abc-def-GHI"),
            "ABC_DEF_GHI".to_string()
        );
        assert_eq!(
            GatewayCmd::kebab_with_leading_dashes_to_screaming_snake("--"),
            "".to_string()
        );
        assert_eq!(
            GatewayCmd::kebab_with_leading_dashes_to_screaming_snake(""),
            "".to_string()
        );
        assert_eq!(
            GatewayCmd::kebab_with_leading_dashes_to_screaming_snake("---"),
            "_".to_string()
        );
        assert_eq!(
            GatewayCmd::kebab_with_leading_dashes_to_screaming_snake("----"),
            "__".to_string()
        );
    }

    #[test]
    fn test_parse_cli_mapping() {
        let args: Vec<String> = Vec::from([
            "--eth-rpc-url".into(),
            "expected eth rpc url".into(),
            "--miner".into(),
            "expected miner".into(),
            "--eth-key-id".into(),
            "expected eth key id".into(),
            "--opf-url".into(),
            "expected opf url".into(),
        ]);

        let unit_under_test = GatewayCmd { args };

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
