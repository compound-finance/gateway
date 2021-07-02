#!/usr/bin/env node
const child_process = require('child_process');
const fs = require('fs').promises;
const path = require('path');
const getopts = require('getopts');
const chalk = require('chalk');
const types = require('@polkadot/types');
const Contract = require('web3-eth-contract');
const os = require('os');

function getEthProvider(network) {
  if (['ropsten', 'goerli'].includes(network)) {
    return `https://${network}-eth.compound.finance`;
  } else {
    throw new Error(`Unknown or unsupported eth network: "${network}"`);
  }
}

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

async function fetchChainDeploymentABI(network) {
  return JSON.parse(await fs.readFile(path.join(__dirname, '..', 'ethereum','networks', `${network}-abi.json`), 'utf8'));
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

async function execGateway(bin, args) {
  console.log(chalk.yellow(`Running Gateway with args ${JSON.stringify(args)}...`));
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

async function setStarports(chainSpec, chainConfig, opts) {
  let chainDeployment = await fetchChainDeployment(chainConfig.eth_network);
  let starportAddress = must(chainDeployment.Contracts, 'Starport');

  // TODO: Enable use case for new chain with starports in genesis.
  //  We are missing deployment block info in (eth) deployment info currently.
  chainSpec.genesis.runtime.palletCash.starports = [`ETH:${starportAddress}`];
}

async function setGenesisConfig(chainSpec, chainConfig, opts) {
  let ethGenesisBlock = chainConfig.eth_genesis_block;

  chainSpec.genesis.runtime.palletCash.genesisBlocks = [{
      Eth: {
        number: ethGenesisBlock.number,
        hash: ethGenesisBlock.hash.slice(2),
        parent_hash: ethGenesisBlock.parent_hash.slice(2),
      }
  }];
}

async function setInitialYield(chainSpec, chainConfig, opts) {
  let chainDeployment = await fetchChainDeployment(chainConfig.eth_network);
  let chainDeploymentABI = await fetchChainDeploymentABI(chainConfig.eth_network);
  let cashTokenAddress = chainDeployment.Contracts.Cash;
  if (!cashTokenAddress) {
    throw new Error(`Missing cash token address for network ${chainConfig.eth_network}`);
  }
  let cashTokenABI = chainDeploymentABI.Cash;
  if (!cashTokenABI) {
    throw new Error(`Missing cash token ABI for network ${chainConfig.eth_network}`);
  }
  let cashToken = new Contract(cashTokenABI, cashTokenAddress);
  cashToken.setProvider(getEthProvider(chainConfig.eth_network));
  let yieldStart = await cashToken.methods.cashYieldStart().call();
  let cashYield = (await cashToken.methods.cashYieldAndIndex().call()).yield;

  chainSpec.genesis.runtime.palletCash.lastYieldTimestamp = Number(yieldStart) * 1000;
  chainSpec.genesis.runtime.palletCash.cashYield = Number(cashYield);
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
  let compoundConfig = await fetchCompoundConfig(chainConfig.eth_network);
  let contracts = upper(compoundConfig['Contracts']);
  let tokenInfos = upper(compoundConfig['Tokens']);

  let rateModels = chainConfig.rate_models;
  let assets = Object.entries(chainConfig.tokens).map(([symbol, info]) => {
    let tokenAddress = contracts[symbol];
    if (!tokenAddress) {
      throw new Error(`Missing contract address on ${chainConfig.eth_network} for ${symbol} from compound-config`);
    }

    let tokenInfo = tokenInfos[symbol];
    if (!tokenInfo) {
      throw new Error(`Missing token info on ${chainConfig.eth_network} for ${symbol} from compound-config`);
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
  let bin;
  if (opts.release) {
    bin = path.join(__dirname, '..', 'releases', opts.release, `gateway-${os.platform()}-${os.arch()}`);
  } else {
    if (!opts.skip_build) {
      await buildGateway();
    }

    bin = process.env['CHAIN_BIN'] || path.join(__dirname, '..', 'target', 'release', 'gateway');
  }

  let chain = opts.chain;

  let chainConfig = JSON.parse(await readFile(chain, 'chain-config.json'));
  let chainSpec = JSON.parse(await execGateway(bin, ["build-spec", "--disable-default-bootnode", "--chain", chainConfig.base_chain]));
  await setAuthorities(chainSpec, chainConfig, opts);
  await setAssetInfo(chainSpec, chainConfig, opts);
  await setReporters(chainSpec, chainConfig, opts);
  await setInitialYield(chainSpec, chainConfig, opts);
  await setStarports(chainSpec, chainConfig, opts);
  await setGenesisConfig(chainSpec, chainConfig, opts);

  await writeFile(chain, 'chain-spec.json', JSON.stringify(chainSpec, null, 2));

  let chainSpecFile = await writeFile(chain, 'chain-spec.json', JSON.stringify(chainSpec, null, 2));

  // Next, build raw spec
  let raw = JSON.parse(await execGateway(bin, ["build-spec", "--chain", chainSpecFile, "--raw", "--disable-default-bootnode"]));

  // Next, build
  await writeFile(chain, 'chain-spec-raw.json', JSON.stringify(raw, null, 2));
}

const options = getopts(process.argv.slice(2), {
  alias: {
    chain: "c",
    skip_build: "s",
    release: "r"
  }
});

if (!options.chain) {
  throw new Error(`Must choose chain with -c`);
}

buildSpec({
  skip_build: false,
  ...options
});
