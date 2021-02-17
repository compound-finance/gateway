
module.exports = {
  solc: "solc",                                         // Solc command to run
  solc_args: [],                                        // Extra solc args
  solc_shell_args: {                                    // Args passed to `exec`, see:
    maxBuffer: 1024 * 5000                              // https://nodejs.org/api/child_process.html#child_process_child_process_spawn_command_args_options
  },
  build_dir: ".build",                                  // Directory to place built contracts
  extra_build_files: [],                                // Additional build files to deep merge
  coverage_dir: "coverage",                             // Directory to place coverage files
  coverage_ignore: [],                                  // List of files to ignore for coverage
  contracts: ["contracts/*.sol",
              "contracts/vendor/**/*.sol",
              "contracts/test/*.sol"].join(" "),        // Glob to match contract files
  tests: ['**/tests/*test.js'],
  trace: false,                                         // Compile with debug artifacts
  networks: {                                           // Define configuration for each network
    goerli: {
      providers: [
        {env: "PROVIDER"},
        {file: "~/.ethereum/goerli-url"},               // Load from given file with contents as the URL (e.g. https://infura.io/api-key)
        {http: "https://goerli.infura.io"}
      ],
      web3: {
        gas: [
          {env: "GAS"},
          {default: "4600000"}
        ],
        gas_price: [
          {env: "GAS_PRICE"},
          {default: "12000000000"}
        ],
        options: {
          transactionConfirmationBlocks: 1,
          transactionBlockTimeout: 5
        }
      },
      accounts: [
        {env: "ACCOUNT"},
        {file: "~/.ethereum/goerli"}                    // Load from given file with contents as the private key (e.g. 0x...)
      ]
    }
  },
  scripts: {
    "deploy": "deploy.js"
  }                                                     // Aliases for scripts
}
