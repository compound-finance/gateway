const path = require('path');
const { merge, sleep } = require('../util');
const { declare } = require('./declare');

const { baseScenInfo } = require('./scen_info');
const { buildEth } = require('./eth');
const { buildCashToken } = require('./cash_token');
const { buildStarport } = require('./starport');
const { buildTokens } = require('./token');
const { buildChainSpec } = require('./chain_spec');
const { buildValidators } = require('./validator');
const { buildActors } = require('./actor');
const { buildTrxReq } = require('./trx_req');
const { buildChain } = require('./chain');
const { buildPrices } = require('./prices');
const { buildVersions } = require('./versions');
const { buildLogger } = require('./logger');
const { buildEventTracker } = require('./event_tracker');

class Ctx {
  constructor(scenInfo) {
    this.scenInfo = scenInfo;
    this.startTime = Math.floor(Date.now() / 1000);
  }

  __startTime() {
    return this.startTime;
  }

  __repoUrl() {
    return process.env['REPO_URL'] || this.scenInfo['repo_url'];
  }

  __initialYield() {
    return process.env['INITIAL_YIELD'] || this.scenInfo['initial_yield'] || 0;
  }

  __initialYieldStart() {
    return process.env['INITIAL_YIELD_START'] || this.scenInfo['initial_yield_start'] || this.startTime;
  }

  __getContractsDir() {
    return process.env['BUILD_DIR'] || this.scenInfo['contracts_dir'] ||
      path.join(__dirname, '..', '..', '..', 'ethereum', '.build');
  }

  __getContractsFile(buildFileName = 'contracts.json') {
    return path.join(this.__getContractsDir(), buildFileName);
  }

  __logLevel() {
    return process.env['LOG'] || this.scenInfo['log_level'];
  }

  __abort(msg) {
    this.error(msg);
    process.exit(1);
  }

  __linkValidator() {
    return process.env['LINK_VALIDATOR'] || this.scenInfo['link_validator'];
  }

  __profile() {
    return process.env['PROFILE'] || this.scenInfo['profile'];
  }

  __target() {
    return process.env['CHAIN_BIN'] || this.scenInfo['target'] || path.join(__dirname, '..', '..', '..', 'target', this.__profile(), 'gateway');
  }

  __wasmFile() {
    return process.env['WASM_FILE'] || this.scenInfo['wasm_file'] || path.join(__dirname, '..', '..', '..', 'target', this.__profile(), 'wbuild', 'gateway-runtime', 'gateway_runtime.compact.wasm');
  }

  __genesisVersion() {
    return process.env['GENESIS_VERSION'] || this.scenInfo['genesis_version'];
  }

  __typesFile() {
    return process.env['TYPES_FILE'] || this.scenInfo['types_file'] || path.join(__dirname, '..', '..', '..', 'types.json');
  }

  __rpcFile() {
    return process.env['RPC_FILE'] || this.scenInfo['rpc_file'] || path.join(__dirname, '..', '..', '..', 'rpc.json');
  }

  __provider() {
    return process.env['PROVIDER'] || this.scenInfo['eth_opts']['provider'];
  }

  __usePriceServer() {
    return !process.env['OPF_URL'];
  }

  __opfUrl() {
    return process.env['OPF_URL'] ? process.env['OPF_URL'] : ( this.prices.serverUrl() || this.scenInfo['opf_url'] );
  }

  __reporters() {
    return process.env['REPORTERS'] ? process.env['REPORTERS'].split(',') : this.scenInfo['reporters'];
  }

  logFile() {
    return process.env['LOG_FILE'] || this.scenInfo['log_file'];
  }

  debug(...msg) {
    this.logger.debug(...msg);
  }

  log(...msg) {
    this.logger.log(...msg);
  }

  error(...msg) {
    this.logger.error(...msg);
  }

  api() {
    return this.validators.api();
  }

  declare(declareInfo, ...args) {
    return declare(this, declareInfo, ...args);
  }

  getTestObject() {
    let tokens = Object.fromEntries(this.tokens.all().map((token) => [token.name, token.token]));
    return {
      accounts: this.eth.accounts,
      ashley: this.actors.get('ashley'),
      api: this.validators.first().api,
      bert: this.actors.get('bert'),
      contracts: {
        tokens: tokens,
        starport: this.starport.starport,
        cashToken: this.cashToken.cashToken,
      },
      ctx: this,
      keyring: this.actors.keyring,
      provider: this.eth.web3.provider,
      web3: this.eth.web3,
    }
  }

  generateTrxReq(...args) {
    return this.trxReq.generate(...args);
  }

  async teardown() {
    if (this.eventTracker) {
      await this.eventTracker.teardown();
    }

    if (this.validators) {
      await this.validators.teardown();
    }

    if (this.eth) {
      await this.eth.teardown();
    }

    if (this.prices) {
      await this.prices.teardown();
    }

    if (this.logger) {
      await this.logger.teardown();
    }

    await sleep(1000); // Give things a second to close
  }
}

function aliasBy(ctx, iterator, key) {
  // Set top-level aliases on ctx
  let setKey = (k, v) => {
    if (!k) {
      throw new Error(`Trying to set ctx value, but k is undefined for ${v}`);
    } else if (ctx[k]) {
      throw new Error(`Trying to duplicate set ctx value: ${k}`);
    } else {
      ctx[k] = v;
    }
  }

  iterator.forEach((el) => {
    setKey(el[key], el);
  });
}

async function buildCtx(scenInfo={}) {
  scenInfo = merge(baseScenInfo, scenInfo);
  let ctx = new Ctx(scenInfo);
  ctx.logger = await buildLogger(ctx);
  ctx.log(`Building ctx with scenInfo=${JSON.stringify(scenInfo, null, 2)}`);
  ctx.log(`test=${JSON.stringify(scenInfo.chain_spec, null, 2)}`);
  ctx.versions = await buildVersions(scenInfo.versions, ctx);
  ctx.eth = await buildEth(scenInfo.eth_opts, ctx);

  // Note: `3` below is the number of transactions we expect to occur between now and when
  //       the Starport token is deployed.
  //       That's now: deploy Proxy Admin (1), Cash Token Impl (2), Starport Impl (3), Proxy (4)
  let starportAddress = await ctx.eth.getNextContractAddress(4);

  ctx.cashToken = await buildCashToken(scenInfo.cash_token, ctx, starportAddress);
  ctx.starport = await buildStarport(scenInfo.starport, scenInfo.validators, ctx);
  ctx.actors = await buildActors(scenInfo.actors, scenInfo.default_actor, ctx);
  ctx.tokens = await buildTokens(scenInfo.tokens, scenInfo, ctx);
  ctx.chainSpec = await buildChainSpec(scenInfo.chain_spec, scenInfo.validators, scenInfo.tokens, ctx);
  ctx.prices = await buildPrices(scenInfo.prices, scenInfo.tokens, ctx);
  ctx.validators = await buildValidators(scenInfo.validators, ctx);
  ctx.trxReq = await buildTrxReq(ctx);
  ctx.chain = await buildChain(ctx);
  ctx.eventTracker = await buildEventTracker(ctx);
  ctx.sleep = sleep;

  // TODO: Post prices?

  aliasBy(ctx, ctx.actors.all(), 'name');
  aliasBy(ctx, ctx.tokens.all(), 'ticker');
  aliasBy(ctx, ctx.validators.all(), 'name');
  aliasBy(ctx, ctx.versions.all(), 'symbolized');

  return ctx;
}

module.exports = {
  Ctx,
  buildCtx,
};
