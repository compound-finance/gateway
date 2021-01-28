const { log } = require('./log');
const fs = require('fs').promises;
const ABI = require('web3-eth-abi');

async function deployContract(web3, from, contracts, contractName, args) {
  log(`Deploying contract ${contractName}`);
  let contractObj = Object.entries(contracts).find(([name, contract]) => name.split(':')[1] === contractName);
  if (!contractObj) {
    throw new Error(`Could not find contract: ${contractName}`);
  }
  let [_, contract] = contractObj;
  let abi = typeof (contract.abi) === 'string' ? JSON.parse(contract.abi) : contract.abi;
  let constructor = abi.find((m) => m.type === 'constructor' && m.inputs.length === args.length);
  if (!constructor) {
    if (abi.filter((m) => m.type === 'constructor').length === 0 && args.length === 0) {
      constructor = { inputs: [] };
    } else {
      throw new Error(`Could not find constructor with length ${args.length} for ${contractName}`);
    }
  }

  let parameters = ABI.encodeParameters(constructor.inputs, args);
  let constructorCall = '0x' + contract.bin + parameters.slice(2);

  let res = await web3.eth.sendTransaction({
    from,
    to: null,
    value: 0,
    gas: 6000000,
    gasPrice: 0,
    data: constructorCall
  });

  return new web3.eth.Contract(abi, res.contractAddress);
}

async function readContractsFile(contractsFile) {
  try {
    return JSON.parse(await fs.readFile(contractsFile, 'utf8')).contracts;
  } catch (e) {
    throw new Error(`Compiled contracts missing-- please run \`yarn compile\` in compound-chain/ethereum directory first. ${e.toString()}`)
  }
}

function getEventValues(event) {
  let returnValues = event.returnValues;
  return Object.entries(returnValues).reduce((acc, [k, v]) => {
    if (Number.isNaN(Number(k))) {
      return {
        ...acc,
        [k]: v
      };
    } else {
      return acc;
    }
  }, {});
}

module.exports = {
  deployContract,
  getEventValues,
  readContractsFile,
};
