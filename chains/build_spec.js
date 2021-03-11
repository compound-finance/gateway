#!env node
const child_process = require('child_process');
const fs = require('fs').promises;
const path = require('path');
const getopts = require('getopts');
const chalk = require('chalk');
const types = require('@polkadot/types');

async function readFile(chain, file) {
  return await fs.readFile(path.join(__dirname, chain, file), 'utf8');
}

async function writeFile(chain, file, contents) {
  let filePath = path.join(__dirname, chain, file);
  await fs.writeFile(filePath, contents);
  return filePath;
}

async function fetchCompoundConfig(network) {
  return JSON.parse(await fs.readFile(path.join(__dirname, 'node_modules', 'compound-config', 'networks', `${network}.json`), 'utf8'));
}

async function fetchChainDeployment(network) {
  return JSON.parse(await fs.readFile(path.join(__dirname, '..', 'ethereum','networks', `${network}.json`), 'utf8'));
}

function exec(target, args, opts = {}) {
  return new Promise((resolve, reject) => {
    let proc = child_process.spawn(target, args, opts);
    let res = "";

    proc.stdout.on('data', (data) => {
      res += data;
    });

    proc.stderr.on('data', (data) => {
      console.log(`[${target}]: ${data}`);
    });

    proc.on('close', (code) => {
      if (code === 0) {
        resolve(res);
      } else {
        reject(`Process \`${target}\` exited with error code: ${code}`);
      }
    });
  });
}

async function buildGateway() {
  console.log(chalk.yellow("Building Gateway release build..."));
  return await exec("cargo", ["build", "--release"]);
}

async function execGateway(args) {
  console.log(chalk.yellow(`Running Gateway with args ${JSON.stringify(args)}...`));
  let bin = process.env['CHAIN_BIN'] || path.join(__dirname, '..', 'target', 'release', 'gateway');
  return await exec(bin, args);
}

async function setAuthorities(chainSpec, chainConfig, opts) {
  let registry = new types.TypeRegistry();
  let validators = chainConfig.validators.map(({substrate_id, eth_address}) => {
    let substrateId = (new types.GenericAccountId(registry, substrate_id));

    return {
      substrate_id: [...substrateId.toU8a()],
      eth_address
    };
  });

  let sessionKeys = chainConfig.validators.filter((s) => s.hasOwnProperty('session_keys')).map(({substrate_id, session_keys}) => {
    return [
      substrate_id,
      substrate_id,
      session_keys
    ];
  });

  chainSpec.genesis.runtime.palletCash.validators = validators;
  chainSpec.genesis.runtime.palletSession.keys = sessionKeys;
}

async function setStarport(chainSpec, chainConfig, opts) {
  let chainDeployment = await fetchChainDeployment(chainConfig.network);
  let starportAddress = must(chainDeployment.Contracts, 'Starport');
  chainSpec.properties.eth_starport_address = starportAddress;
}

function must(hash, key, validator = (x) => x !== undefined) {
  if (hash.hasOwnProperty(key) || validator(undefined)) {
    let val = hash[key];
    if (!validator(val)) {
      throw new Error(`Error validating ${key} with val ${val}`);
    }
    return val;
  } else {
    throw new Error(`Could not find key ${key} for object with keys ${JSON.stringify(Object.keys(hash))}`);
  }
}

function upper(obj) {
  return Object.fromEntries(Object.entries(obj).map(([k, v]) => [k.toUpperCase(), v]))
}

async function setAssetInfo(chainSpec, chainConfig, opts) {
  let compoundConfig = await fetchCompoundConfig(chainConfig.network);
  let contracts = upper(compoundConfig['Contracts']);
  let tokenInfos = upper(compoundConfig['Tokens']);

  let rateModels = chainConfig.rate_models;
  let assets = Object.entries(chainConfig.tokens).map(([symbol, info]) => {
    let tokenAddress = contracts[symbol];
    if (!tokenAddress) {
      throw new Error(`Missing contract address on ${chainConfig.network} for ${symbol} from compound-config`);
    }

    let tokenInfo = tokenInfos[symbol];
    if (!tokenInfo) {
      throw new Error(`Missing token info on ${chainConfig.network} for ${symbol} from compound-config`);
    }
    let asset = `ETH:${tokenAddress}`;
    let decimals = must(tokenInfo, 'decimals', (d) => typeof(d) === 'number');
    let liquidity_factor = must(info, 'liquidity_factor', (d) => typeof(d) === 'number' && d > 0 && d <= 1) * 1e18;
    let rate_model;
    let rateModelRaw = must(info, 'rate_model');
    if (typeof(rateModelRaw) === 'string') {
      rate_model = rateModels[rateModelRaw];
      if (!rate_model) {
        throw new Error(`Unknown or missing rate model: ${rate_model}`)
      }
    } else {
      rate_model = rateModelRaw;
    }
    let miner_shares = must(info, 'miner_shares', (d) => typeof(d) === 'number' && d >= 0 && d <= 1) * 1e18;
    let supply_cap = must(info, 'supply_cap', (d) => typeof(d) === 'undefined' || typeof(d) === 'number') || 0;
    let ticker = must(info, 'ticker', (d) => typeof(d) === 'undefined' || typeof(d) === 'string') || symbol;

    return {
      asset,
      decimals,
      liquidity_factor,
      rate_model,
      miner_shares,
      supply_cap,
      symbol,
      ticker
    };
  });

  chainSpec.genesis.runtime.palletCash.assets = assets;
}

async function setReporters(chainSpec, chainConfig, opts) {
  chainSpec.genesis.runtime.palletOracle.reporters = chainConfig.reporters;
}

async function buildSpec(opts) {
  if (!opts.skip_build) {
    await buildGateway();
  }
  let chain = opts.chain;

  let chainConfig = JSON.parse(await readFile(chain, 'chain-config.json'));
  let chainSpec = JSON.parse(await execGateway(["build-spec", "--disable-default-bootnode", "--chain", chainConfig.base_chain]));
  await setAuthorities(chainSpec, chainConfig, opts);
  await setAssetInfo(chainSpec, chainConfig, opts);
  await setReporters(chainSpec, chainConfig, opts);
  await setStarport(chainSpec, chainConfig, opts);

  await writeFile(chain, 'chain-spec.json', JSON.stringify(chainSpec, null, 2));

  let chainSpecFile = await writeFile(chain, 'chain-spec.json', JSON.stringify(chainSpec, null, 2));

  // Next, build raw spec
  let raw = JSON.parse(await execGateway(["build-spec", "--chain", chainSpecFile, "--raw", "--disable-default-bootnode"]));

  // Next, build
  await writeFile(chain, 'chain-spec-raw.json', JSON.stringify(raw, null, 2));
}

const options = getopts(process.argv.slice(2), {
  alias: {
    chain: "c",
    skip_build: "s"
  }
});

if (!options.chain) {
  throw new Error(`Must choose chain with -c`);
}

buildSpec({
  skip_build: false,
  ...options
});
