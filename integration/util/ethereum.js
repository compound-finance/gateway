const path = require('path');
const fs = require('fs').promises;
const ABI = require('web3-eth-abi');
const { log, error } = require('./log');

let contractsFile = path.join(__dirname, '..', '..', 'ethereum', '.build', 'contracts.json');

async function deployContract(web3, from, contracts, contractName, args) {
  let contractObj = Object.entries(contracts).find(([name, contract]) => name.split(':')[1] === contractName);
  if (!contractObj) {
    throw new Error(`Could not find contract: ${contractName}`);
  }
  let [_, contract] = contractObj;
  let abi = JSON.parse(contract.abi);
  let constructor = abi.find((m) => m.type === 'constructor' && m.inputs.length === args.length);
  if (!constructor) {
    throw new Error(`Could not find constructor with length ${args.length} for ${contractName}`);
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

async function deployContracts(web3) {
  let contracts;
  try {
    contracts = JSON.parse(await fs.readFile(contractsFile, 'utf8')).contracts;
  } catch (e) {
    throw new Error(`Compiled contracts missing-- please run \`yarn compile\` in compound-chain/ethereum directory first. ${e.toString()}`)
  }

  let accounts = await web3.eth.personal.getAccounts();
  log("Deploying cash token...");
  let cashToken = await deployContract(web3, accounts[0], contracts, 'CashToken', [accounts[0]]);
  log("Deploying starport...");
  let starport = await deployContract(web3, accounts[0], contracts, 'Starport', [cashToken._address, accounts]);

  log(`CashToken=${cashToken._address}, Starport=${starport._address}`);

  return {
    cashToken,
    starport
  };
}

module.exports = { deployContracts };
