const ganache = require('ganache-core');
const Web3 = require('web3');
const { readContractsFile, deployContract } = require('../ethereum');
const { genPort } = require('../util');

class Eth {
  constructor(ethInfo, web3, web3Url, accounts, ganacheServer, ctx) {
    this.ethInfo = ethInfo;
    this.web3 = web3;
    this.web3Url = web3Url;
    this.accounts = accounts;
    this.defaultFrom = accounts[0];
    this.ganacheServer = ganacheServer;
    this.ctx = ctx;
  }

  async __deployContract(contractsFile, contractName, contractArgs, opts = {}) {
    let contracts = await readContractsFile(contractsFile);

    let contract = await deployContract(
      this.web3,
      opts.from || this.defaultFrom,
      contracts,
      contractName,
      contractArgs
    );

    this.ctx.log(`${contractName} deployed to ${contract._address}`);

    return contract;
  }

  async sign(data, actorLookup) {
    let actor = this.ctx.actors.get(actorLookup);

    return this.web3.eth.sign(data, actor.ethAddress());
  }

  async ethBalance(actorLookup) {
    let actor = this.ctx.actors.get(actorLookup);

    return Number(await this.web3.eth.getBalance(actor.ethAddress()));
  }

  async teardown() {
    if (this.ganacheServer) {
      await this.ganacheServer.close(); // Close ganache server
    }
  }
}

async function buildEth(ethInfo, ctx) {
  let provider = ctx.__provider() || ethInfo.provider;
  let web3;
  let ganacheServer; // Keep track for teardown
  let web3Url;

  if (provider === 'ganache') {
    ganacheServer = ganache.server(ethInfo.ganache.opts);
    let ganacheProvider = ganacheServer.provider;

    web3Port = ethInfo.ganache.web3_port || genPort();
    web3Url = `http://localhost:${web3Port}`;

    // Start web3 server
    ctx.log(`Starting Ethereum server on ${web3Port}...`);
    ganacheServer.listen(web3Port);

    web3 = new Web3(ganacheProvider, null, { transactionConfirmationBlocks: 1 });
  } else {
    web3Url = provider;
    web3 = new Web3(provider);
  }

  // We'll enumerate accounts early so we don't need to repeat often.
  let accounts = await web3.eth.personal.getAccounts();

  return new Eth(ethInfo, web3, web3Url, accounts, ganacheServer, ctx);
}

module.exports = {
  buildEth,
  Eth
};
