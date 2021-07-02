const ganache = require('ganache-core');
const Web3 = require('web3');
const RLP = require('rlp');
const { readContractsFile, deployContract, getContractAt } = require('../ethereum');
const { genPort } = require('../util');

class BlockInfo {
  constructor(web3, blockNumber, ctx) {
    this.web3 = web3;
    this.ctx = ctx;
    this.hash = null;
    this.parentHash = null;
    this.number = null;
    this.blockNumber = blockNumber;
  }

  async update() {
    let block = await this.web3.eth.getBlock(this.blockNumber);

    this.hash = block.hash;
    this.parentHash = block.parentHash;
    this.number = block.number;
  }
}

class Chain {
  constructor(chainInfo, web3, web3Url, accounts, blockInfo, ganacheServer, version, ctx) {
    this.chainInfo = chainInfo;
    this.name = chainInfo.name;
    this.web3 = web3;
    this.web3Url = web3Url;
    this.accounts = accounts;
    this.defaultFrom = accounts[0];
    this.blockInfo = blockInfo;
    this.ganacheServer = ganacheServer;
    this.version = version;
    this.ctx = ctx;
    this.contractsFiles = {};
    this.asyncId = 1;
  }

  root() {
    return this.defaultFrom;
  }

  async updateGenesisBlock() {
    await this.blockInfo.update();
  }

  nameAsPascalCase() {
    return this.name.charAt(0).toUpperCase() + this.name.slice(1);
  }

  nameAsUpperCase() {
    return this.name.toUpperCase();
  }

  // first 3 chars then a : - for matic - MAT:
  nameAsStarportHeader() {
    return this.nameAsUpperCase().substring(0, 3) + ":"
  }

  genesisBlock() {
    return {
      [this.nameAsPascalCase()]: {
        hash: this.blockInfo.hash,
        parentHash: this.blockInfo.parentHash,
        number: this.blockInfo.number,
        events: []
      }
    };
  }

  genesisBlockForChainSpec() {
    const block = this.genesisBlock();
    const name = this.nameAsPascalCase();
    return {
      [name]: {
        hash: block[name].hash.slice(2),
        parent_hash: block[name].parentHash.slice(2),
        number: block[name].number,
      }
    };
  }


  sendAsync(method, params = []) {
    let id = this.asyncId++;

    return new Promise((resolve, reject) => {
      this.web3.currentProvider.sendAsync({
        jsonrpc: "2.0",
        method,
        params,
        id: id
      }, function(err, result) {
        if (err) {
          reject(err);
        } else {
          if (result.id !== id) {
            throw new Error(`Incorrect response id. Expected=${id}, Received=${result.id}`);
          }

          resolve(result.result);
        }
      });
    });
  }

  async mine(count = 1, ts = undefined) {
    for (const i in [...new Array(count)]) {
      let params = [ts].filter((x) => x !== undefined);
      await this.sendAsync('evm_mine', params);
    }
  }

  async stopMining() {
    await this.sendAsync('miner_stop', []);
  }

  async startMining() {
    await this.sendAsync('miner_start', []);
  }

  async snapshot() {
    return await this.sendAsync('evm_snapshot');
  }

  async restore(snapshotId) {
    await this.sendAsync('evm_revert', [snapshotId]);
  }

  async getContractsFile(contractsFile) {
    if (this.contractsFiles[contractsFile]) {
      return this.contractsFiles[contractsFile];
    } else {
      let result = await readContractsFile(contractsFile);
      this.contractsFiles[contractsFile] = result;
      return result;
    }
  }

  async __deploy(contractName, contractArgs, opts = {}) {
    opts = {
      version: this.version,
      ...opts
    };
    return await this.__deployFull(opts.version.contractsFile(), contractName, contractArgs, opts);
  }

  async __deployFull(contractsFile, contractName, contractArgs, opts = {}) {
    opts = {
      from: this.defaultFrom,
      ...opts
    };
    this.ctx.log("Deploying " + contractName + " from " + contractsFile)
    let contracts = await this.getContractsFile(contractsFile);

    let contract = await deployContract(
      this.web3,
      opts.from,
      contracts,
      contractName,
      contractArgs
    );

    this.ctx.log(`${contractName} deployed to ${contract._address} with args ${JSON.stringify(contractArgs)}`);

    return contract;
  }

  async __getContractAt(contractName, contractAddress, opts = {}) {
    opts = {
      version: this.version,
      ...opts
    };
    return await this.__getContractAtFull(opts.version.contractsFile(), contractName, contractAddress, opts);
  }

  async __getContractAtFull(contractsFile, contractName, contractAddress) {
    let contracts = await this.getContractsFile(contractsFile);

    return getContractAt(this.web3, contracts, contractName, contractAddress);
  }

  __getContractAtAbi(abi, contractAddress) {
    return new this.web3.eth.Contract(abi, contractAddress);
  }

  async sign(data, actorLookup) {
    let actor = this.ctx.actors.get(actorLookup);

    return this.web3.eth.sign(data, actor.ethAddress());
  }

  async ethBalance(actorLookup) {
    let ethAddress;
    if (typeof(actorLookup) === 'string' && actorLookup.slice(0, 2) === '0x') {
      ethAddress = actorLookup;
    } else {
      let actor = this.ctx.actors.get(actorLookup);
      ethAddress = actor.ethAddress();
    }

    return Number(await this.web3.eth.getBalance(ethAddress));
  }

  async getNextContractAddress(skip = 0) {
    const nonce = await this.web3.eth.getTransactionCount(this.defaultFrom);
    const address = this.web3.utils.sha3(
      RLP.encode([this.defaultFrom, nonce + skip])).slice(12).substring(14);
    return this.web3.utils.toChecksumAddress(`0x${address}`);
  }

  async timestamp() {
    return (await this.web3.eth.getBlock("pending")).timestamp;
  }

  async getBlock(number) {
    return (await this.web3.eth.getBlock(number));
  }

  async proxyRead(proxy, field) {
    let hash = {
      implementation: '0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc',
      admin: '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103'
    }[field];
    if (!hash) {
      throw new Error(`unknown proxy read field: ${field}`);
    }

    return await this.web3.eth.getStorageAt(proxy._address, hash);
  }

  async teardown() {
    if (this.ganacheServer) {
      await this.ganacheServer.close(); // Close ganache server
    }
  }
}

class Chains {
  constructor(chains, ctx) {
    this.chains = chains;
    this.ctx = ctx;
  }

  all() {
    return Object.values(this.chains);
  }

  find(chainName) {
    return this.chains[chainName]
  }

  attachDeployments(deployments) {
    deployments.all().forEach(deployment => {
      deployment.chain.starport = deployment.starport;
      deployment.chain.cashToken = deployment.cashToken;
    });
  }

  genesisBlocksForChainSpec() {
    return this.all().map(chain => chain.genesisBlockForChainSpec());
  }
}

async function buildChain(chainInfo, ctx) {
  let provider = chainInfo.provider;
  let web3;
  let ganacheServer; // Keep track for teardown
  let web3Url;
  let accounts;

  // XXX this doesn't generalize well to chainInfo though actually right?
  if (provider === 'ganache') {
    let ganacheOpts = chainInfo.ganache.opts;
    if (ctx.__blockTime() !== null) {
      ganacheOpts.blockTime = ctx.__blockTime();
    }

    ganacheServer = ganache.server(ganacheOpts);
    let ganacheProvider = ganacheServer.provider;

    let web3Port = chainInfo.ganache.web3_port || genPort();
    web3Url = `http://localhost:${web3Port}`;

    // Start web3 server
    ctx.log(`Starting Ethereum server on ${web3Port}...`);
    ganacheServer.listen(web3Port);

    web3 = new Web3(ganacheProvider, null, { transactionConfirmationBlocks: 1 });

    // We'll enumerate accounts early so we don't need to repeat often.
    accounts = await web3.eth.personal.getAccounts();
  } else {
    web3Url = provider;
    web3 = new Web3(provider);
    accounts = ["0x0000000000000000000000000000000000000000"];
  }

  let version = chainInfo.version ? ctx.versions.mustFind(chainInfo.version) : ctx.versions.current;

  let blockInfo = new BlockInfo(web3, chainInfo.block_number, ctx);
  await blockInfo.update();

  return new Chain(chainInfo, web3, web3Url, accounts, blockInfo, ganacheServer, version, ctx);
}

async function buildChains(chainsInfo, ctx) {
  const chainsList = await Promise.all(
      Object.values(chainsInfo)
          .map(chainInfo => buildChain(chainInfo, ctx))
  )
  // maybe a clearer/easier way to do that
  const chains = chainsList.reduce((acc, chain) => {acc[chain.name] = chain; return acc;}, {})

  return new Chains(chains, ctx);
}

module.exports = {
  buildChain,
  buildChains,
  Eth: Chain
};
