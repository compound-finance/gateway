const RLP = require('rlp');
const fs = require('fs').promises;
const { constants } = require('fs');
const path = require('path');
const chalk = require('chalk');

async function deployAndVerify(contract, args, opts, saddle, env, network) {
  let etherscan = env['etherscan'];

  // Delay for Etherscan to pick up contract
  let etherscanDelay = env['etherscan_delay'] ? Number(env['etherscan_delay']) : 35_000;

  console.log(`Deploying ${chalk.blue(chalk.bold(contract))} with args ${chalk.green(JSON.stringify(args))}`);
  let res = await saddle.deploy(contract, args, opts);
  console.log(`Deployed ${contract} to ${res._address} [View on Etherscan](https://${network}.etherscan.io/address/${res._address})\n`);
  if (etherscan && network !== 'development') {
    await sleep(etherscanDelay); // Give Etherscan time to pick up the contract

    try {
      console.log(`Verifying ${contract} on Etherscan...`);
      await saddle.verify(etherscan, res._address, contract, args);
    } catch (e) {
      console.log(chalk.yellow(`Failed to verify on Etherscan: ${e}`));
    }
  }
  return res;
}

const getNextContractAddress = (acct, nonce, web3) => {
  return '0x' + web3.utils.sha3(RLP.encode([acct, nonce])).slice(12).substring(14);
};

async function fileExists(path) {
  try {
    await fs.access(path, constants.R_OK);
    return true;
  } catch (e) {
    return false;
  }
}

async function writeJSON(file, data) {
  await fs.writeFile(path.join(__dirname, '..', '..', file), JSON.stringify(data, null, 4));
}

async function readJSON(file, data) {
  if (await fileExists(file)) {
    return await JSON.parse(await fs.readFile(path.join(__dirname, '..', '..', file), 'utf8'));
  } else {
    return {};
  }
}

// From https://github.com/jonschlinkert/isobject/blob/master/index.js
function isObject(val) {
  return val != null && typeof val === 'object' && Array.isArray(val) === false;
};

// Merges objects together but not arrays, etc.
function semiDeepMerge(a, b) {
  return Object.entries(b).reduce((acc, [k, v]) => {
    let curr = acc[k];
    let res = isObject(curr) && isObject(v) ? semiDeepMerge(curr, v) : v;
    return {
      ...acc,
      [k]: res
    }
  }, a);
}

async function mergeJSON(file, data) {
  let json = await readJSON(file);
  let result = semiDeepMerge(json, data);
  await writeJSON(file, result);
}

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function checkAddress(address) {
  if (!!address && address.startsWith("0x") && address.length == "42") {
    return;
  } else {
    throw new Error(`Invalid address: ${address}`);
  }
}

const fromNow = (seconds) => {
  return Math.floor(seconds + (Date.now() / 1000));
}

const mapValues = (obj, fn) => {
  return Object.fromEntries(Object.entries(obj).map(([k, v]) => [k, fn(v)]))
}

const readNetwork = async (saddle, _env, network) => {
  return await readJSON(`networks/${network}.json`);
}

const saveNetwork = async (contracts, saddle, _env, network) => {
  const showContracts = Object.entries(contracts).map(([name, contract]) =>
    `\t${chalk.bold(name)}: ${contract._address}`).join("\n");

  console.log(`\n${chalk.blue("Deployed")}: \n${showContracts}`);

  await mergeJSON(`networks/${network}.json`, {
    Contracts: mapValues(contracts, (contract) => contract._address)
  });

  await mergeJSON(
    `networks/${network}-abi.json`,
    mapValues(contracts, (contract) => contract._jsonInterface)
  );
}

module.exports = {
  deployAndVerify,
  getNextContractAddress,
  writeJSON,
  sleep,
  checkAddress,
  fromNow,
  readNetwork,
  saveNetwork
};
