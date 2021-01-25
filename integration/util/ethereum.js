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

let getTokenInfo = (accounts) => ({
  ZRX: {
    build: 'zrx.json',
    contract: 'ZRXToken',
    constructorArgs: []
  },
  DAI: {
    build: 'dai.json',
    contract: 'Dai',
    constructorArgs: [0] // TODO: ChainId
  },
  COMP: {
    build: 'compound.json',
    contract: 'Comp',
    constructorArgs: [accounts[0]]
  },
  BAT: {
    build: 'bat.json',
    contract: 'BAToken',
    constructorArgs: ['0x0000000000000000000000000000000000000000', accounts[0], 0, 0]
  },
  WBTC: {
    build: 'wbtc.json',
    contract: 'WBTC',
    constructorArgs: []
  },
  USDC: {
    build: 'FiatTokenV1.json',
    contract: 'FiatTokenV1',
    constructorArgs: [],
    afterDeploy: async (contract, [owner, ..._accounts]) => {
      await contract.methods.initialize(
        "USD Coin",
        "USDC",
        "USD",
        6,
        owner,
        owner,
        owner,
        owner
      ).send({ from: owner, gas: 5000000 }); // Note: default gas is too low for this function
    }
  }
});

async function deployContracts(web3, validators) {
  let contracts;
  try {
    contracts = JSON.parse(await fs.readFile(contractsFile, 'utf8')).contracts;
  } catch (e) {
    throw new Error(`Compiled contracts missing-- please run \`yarn compile\` in compound-chain/ethereum directory first. ${e.toString()}`)
  }

  let accounts = await web3.eth.personal.getAccounts();

  log("Deploying Erc20 Tokens...");
  let tokens = await Object.entries(getTokenInfo(accounts)).reduce(async (accP, [name, info]) => {
    let acc = await accP;

    log(`Deploying ${name}...`);
    let contractFile = await fs.readFile(path.join(__filename, '..', '..', '..', 'ethereum', '.build', info.build), 'utf8');
    let buildFile = JSON.parse(contractFile).contracts;
    let token = await deployContract(web3, accounts[0], buildFile, info.contract, info.constructorArgs);
    if (typeof (info.afterDeploy) === 'function') {
      await info.afterDeploy(token, accounts);
    }

    return {
      ...acc,
      [name]: token
    };
  }, Promise.resolve({}));

  log("Deploying cash token...");
  let cashToken = await deployContract(web3, accounts[0], contracts, 'CashToken', [accounts[0]]);
  log("Deploying starport...");
  let starport = await deployContract(web3, accounts[0], contracts, 'Starport', [cashToken._address, validators]);

  let tokenStr = Object.entries(tokens).map(([name, contract]) => `${name}=${contract._address}`);
  log([
    `CashToken=${cashToken._address}`,
    `Starport=${starport._address}`,
    ...tokenStr
  ].join(', '));

  return {
    cashToken,
    starport,
    tokens
  };
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
  deployContracts,
  getEventValues
};
