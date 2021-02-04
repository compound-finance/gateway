const os = require('os');
const path = require('path');
const fs = require('fs').promises;
const util = require('util');
const child_process = require('child_process');
const execFile = util.promisify(child_process.execFile);
const { merge, stripHexPrefix } = require('../util');
const { getValidatorsInfo } = require('./validator');

class ChainSpec {
  constructor(chainSpecInfo, chainSpecFile, ctx) {
    this.chainSpecInfo = chainSpecInfo;
    this.chainSpecFile = chainSpecFile;
    this.ctx = ctx;
  }

  file() {
    return this.chainSpecFile;
  }
}

async function baseChainSpec(validatorsInfoHash, tokensInfoHash, ctx) {
  let tokens = ctx.tokens;
  if (!tokens) {
    throw new Error(`Tokens required to build chain spec`);
  }

  let validatorsInfo = await getValidatorsInfo(validatorsInfoHash, ctx);

  // aurakey == validator id, account id
  let session_args = validatorsInfo.map(([_, el]) => [el.aura_key, el.aura_key, {aura: el.aura_key, grandpa: el.grandpa_key}]);

  let validatorIds = validatorsInfo.map(([_, el]) =>
    Array.from(ctx.actors.keyring.decodeAddress(el.aura_key))// from ss58 str => byte array
  );

  let chain_keys = validatorsInfo.map(([_, validator]) =>
    [stripHexPrefix(validator.eth_account)]
  );
  let assets = tokens.all().map((token) => ({
    symbol: `${token.symbol.toUpperCase()}/${token.decimals}`,
    asset: `eth:${token.ethAddress()}`
  }));

  return {
    name: 'Integration Test Network',
    properties: {
      eth_starport_address: ctx.starport.ethAddress(),
      eth_lock_event_topic: ctx.starport.topics()['Lock']
    },
    genesis: {
      runtime: {
        palletCash: {
          validatorIds,
          validatorKeys: chain_keys,
          assets
        },
        palletSession: {
          keys: session_args
        }
      }
    }
  };
}

async function tmpFile(name) {
  folder = await fs.mkdtemp(path.join(os.tmpdir()));
  return path.join(folder, name);
}

// TODO: Some things here probably need to be cleaned up
async function buildChainSpec(chainSpecInfo, validatorsInfoHash, tokenInfoHash, ctx) {
  let chainSpecFile = chainSpecInfo.use_temp ?
    await tmpFile('chainSpec.json') : path.join(__dirname, '..', '..', 'chainSpec.json');
  let target = ctx.__target();
  ctx.log('Building chain spec from ' + target + ' to temp file ' + chainSpecFile);
  let chainSpecJson;
  try {
    let { error, stdout, stderr } =
      await execFile(target, [
        "build-spec",
        "--disable-default-bootnode",
        "--chain",
        chainSpecInfo.base_chain
      ], { maxBuffer: 100 * 1024 * 1024 }); // 100MB
    chainSpecJson = stdout;
  } catch (e) {
    ctx.__abort(`Failed to spawn validator node. Try running \`cargo build --release\` (Missing ${target}, error=${e})`);
  }

  let originalChainSpec = JSON.parse(chainSpecJson);
  let standardChainSpec = await baseChainSpec(validatorsInfoHash, tokenInfoHash, ctx);
  let finalChainSpec = merge(merge(originalChainSpec, standardChainSpec), chainSpecInfo.props);
  await fs.writeFile(chainSpecFile, JSON.stringify(finalChainSpec, null, 2), 'utf8');

  return new ChainSpec(chainSpecInfo, chainSpecFile, ctx);
}

module.exports = {
  buildChainSpec,
  ChainSpec
};
