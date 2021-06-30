const os = require('os');
const path = require('path');
const fs = require('fs').promises;
const util = require('util');
const child_process = require('child_process');
const execFile = util.promisify(child_process.execFile);
const { merge } = require('../util');
const { getValidatorsInfo } = require('./validator');

class ChainSpec {
  constructor(chainSpecInfo, validatorsInfoHash, tokenInfoHash, chainSpecFile, ctx) {
    this.chainSpecInfo = chainSpecInfo;
    this.validatorsInfoHash = validatorsInfoHash;
    this.tokenInfoHash = tokenInfoHash;
    this.chainSpecFile = chainSpecFile;
    this.ctx = ctx;
    this.codeSubstitutes = {};
  }

  file() {
    return this.chainSpecFile;
  }

  async addSubstitute(block, version) {
    this.codeSubstitutes[`0x${block.toString(16)}`] = await version.wasm();
  }

  async generate() {
    let target = this.ctx.genesisVersion().targetFile();
    this.ctx.log('[CHAINSPEC] Scenario chain_spec.json: ' + this.chainSpecFile);
    let chainSpecJson;
    try {
      let { error, stdout, stderr } =
        await execFile(target, [
          "build-spec",
          "--disable-default-bootnode",
          "--chain",
          this.chainSpecInfo.base_chain
        ], { maxBuffer: 100 * 1024 * 1024 }); // 100MB
      chainSpecJson = stdout;
    } catch (e) {
      this.ctx.__abort(`Failed to spawn validator node. Try running \`cargo build --release\` (Missing ${target}, error=${e})`);
    }

    let originalChainSpec = JSON.parse(chainSpecJson);
    let standardChainSpec = await baseChainSpec(this.codeSubstitutes, this.validatorsInfoHash, this.tokenInfoHash, this.ctx);
    let finalChainSpec = merge(merge(originalChainSpec, standardChainSpec), this.chainSpecInfo.props);
    await fs.writeFile(this.chainSpecFile, JSON.stringify(finalChainSpec, null, 2), 'utf8');
  }
}

async function baseChainSpec(codeSubstitutes, validatorsInfoHash, tokensInfoHash, ctx) {
  let tokens = ctx.tokens;
  if (!tokens) {
    throw new Error(`Tokens required to build chain spec`);
  }

  let validatorsInfo = await getValidatorsInfo(validatorsInfoHash, ctx);

  // aurakey == validator id, account id
  let session_args = validatorsInfo.map(([_, v]) => [v.aura_key, v.aura_key, {aura: v.aura_key, grandpa: v.grandpa_key}]);
  let validators = validatorsInfo.filter(([_, v]) => v.validator).map(([_, v]) => ({
      substrate_id: Array.from(ctx.actors.keyring.decodeAddress(v.aura_key)), // from ss58 str => byte array
      eth_address: v.eth_account
  }));

  let assets = tokens.all()
      .filter(token => token.symbol.toUpperCase() !== 'CASH')
      .map((token) => ({
    asset: token.toTrxArg(),
    decimals: token.decimals,
    symbol: token.symbol.toUpperCase(),
    ticker: token.priceTicker,
    liquidity_factor: Math.floor(token.liquidityFactor * 1e18),
    rate_model: {
      Kink: {
        zero_rate: 0,
        kink_rate: 500,
        kink_utilization: 8000,
        full_rate: 2000
      }
    },
    miner_shares: 0,
    supply_cap: 0
  }));

  let initialYieldConfig = {};
  if (ctx.__initialYield() > 0) {
    initialYieldConfig = {
      cashYield: ctx.__initialYield(),
      lastYieldTimestamp: ctx.__initialYieldStartMS()
    };
  }

  let genesisVersion = ctx.genesisVersion();
  let frameSystem = {
    frameSystem: {
      code: await genesisVersion.wasm()
    }
  };

  let reporters = ctx.__reporters();

  let cashGenesis = {};
  if (genesisVersion.supports('starport-parent-block')) {
    cashGenesis.genesisBlocks = ctx.chains.genesisBlocksForChainSpec();
    cashGenesis.starports = ctx.deployments.starportsForChainSpec();
  }

  let extraSpec = {};
  if (Object.keys(codeSubstitutes).length > 0) {
    extraSpec.codeSubstitutes = codeSubstitutes;
  }

  return {
    name: 'Integration Test Network',
    genesis: {
      runtime: {
        ...frameSystem,
        palletCash: {
          assets,
          ...initialYieldConfig,
          validators,
          ...cashGenesis
        },
        palletSession: {
          keys: session_args
        },
        palletOracle: {
          reporters
        }
      }
    },
    ...extraSpec
  };
}

async function tmpFile(name) {
  let folder = await fs.mkdtemp(path.join(os.tmpdir()));
  return path.join(folder, name);
}

// TODO: Some things here probably need to be cleaned up
async function buildChainSpec(chainSpecInfo, validatorsInfoHash, tokenInfoHash, ctx) {
  let chainSpecFile = chainSpecInfo.use_temp ?
    await tmpFile('chainSpec.json') : path.join(__dirname, '..', '..', 'chainSpec.json');

  let chainSpec = new ChainSpec(chainSpecInfo, validatorsInfoHash, tokenInfoHash, chainSpecFile, ctx);

  await chainSpec.generate();

  return chainSpec;
}

module.exports = {
  buildChainSpec,
  ChainSpec
};
