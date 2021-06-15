const path = require('path');
const os = require('os');
const { merge } = require('../util');
const { declare } = require('./declare');

const { baseScenInfo } = require('./scen_info');
const { buildChains } = require('./chains');
const { buildCashToken } = require('./cash_token');
const { buildStarport } = require('./starport');
const { buildDeployments } = require('./deployment');
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
    this.startTime = Date.now();
    this.sleeps = [];
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

  __initialYieldStartRaw() {
    return process.env['INITIAL_YIELD_START'] || this.scenInfo['initial_yield_start'] || this.startTime;
  }

  __initialYieldStartMS() {
    let raw = this.__initialYieldStartRaw();
    if (raw > 4102444800) { // Jan 1, 2100 as seconds since 1970
      // Time is in ms
      return raw;
    } else {
      return raw * 1000;
    }
  }

  __initialYieldStart() {
    return Math.floor(this.__initialYieldStartMS() / 1000);
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

  __deepColor() {
    return !!process.env['DEEP_COLOR'];
  }

  __abort(msg) {
    this.error(msg);
    throw(msg);
  }

  __linkValidator() {
    return process.env['LINK_VALIDATOR'] || this.scenInfo['link_validator'];
  }

  __profile() {
    return process.env['PROFILE'] || this.scenInfo['profile'];
  }

  __buildTarget() {
    return process.env['CHAIN_BIN'] || this.scenInfo['target'] || path.join(__dirname, '..', '..', '..', 'target', this.__profile(), 'gateway');
  }

  __wasmFile() {
    return process.env['WASM_FILE'] || this.scenInfo['wasm_file'] || path.join(__dirname, '..', '..', '..', 'target', this.__profile(), 'wbuild', 'gateway-runtime', 'gateway_runtime.wasm');
  }

  __native() {
    // Note: currently freeze time requires native.
    return this.__freezeTime() || process.env['NATIVE'] || this.scenInfo['native'] || false;
  }

  __freezeTime() {
    let value = process.env['FREEZE_TIME'] || this.scenInfo['freeze_time'];
    if (value !== undefined && value !== null) {
      if (value === true || value === "true") {
        return 0;
      } else {
        let freezeTime = Number(value);
        if (Number.isNaN(freezeTime)) {
          throw new Error(`Invalid freeze time: ${value}`);
        }
        return freezeTime;
      }
    }
    return null;
  }

  __genesisVersion() {
    return process.env['GENESIS_VERSION'] || this.scenInfo['genesis_version'];
  }

  genesisVersion() {
    return this.versions.mustFind(this.__genesisVersion() || 'curr');
  }

  __blockTime() {
    return process.env['BLOCK_TIME'] || this.scenInfo['block_time'];
  }

  __typesFile() {
    return process.env['TYPES_FILE'] || this.scenInfo['types_file'] || path.join(__dirname, '..', '..', '..', 'types.json');
  }

  // TODO: Continue to support extra types
  __types() {
    return this.scenInfo['types'] || undefined;
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

  getApi() {
    return this.validators.api();
  }

  tryApi() {
    return this.validators.tryApi();
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

  __sleep(ms) {
    let resolve, timerId;
    let promise = new Promise((resolve_, reject_) => {
      resolve = resolve_;
      timerId = setTimeout(resolve_, ms)
    });
    this.sleeps.push([timerId, resolve]);
    return promise;
  }

  async __until(cond, opts = {}) {
    let options = {
      delay: 5000,
      retries: null,
      message: null,
      console: false,
      ...opts
    };

    let start = +new Date();

    if (await cond()) {
      return;
    } else {
      if (options.message) {
        let msg = typeof(options.message) === 'function' ? await options.message() : options.message;
        if (console) {
          console.log(msg);
        } else {
          this.log(msg);
        }
      }
      await this.__sleep(options.delay + start - new Date());
      return await this.until(cond, {
        ...options,
        retries: options.retries === null ? null : options.retries - 1
      });
    }
  }

  async teardown() {
    this.sleeps.forEach(([timerId, resolve]) => {
      clearTimeout(timerId)
      resolve();
    });

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

    await this.sleep(1000); // Give things a second to close
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
  ctx.until = ctx.__until.bind(ctx);
  ctx.logger = await buildLogger(ctx);
  ctx.log(`Building ctx with scenInfo=${JSON.stringify(scenInfo, null, 2)}`);
  ctx.log(`test=${JSON.stringify(scenInfo.chain_spec, null, 2)}`);
  ctx.versions = await buildVersions(scenInfo.versions, ctx);
  ctx.chains = await buildChains(scenInfo.chain_opts, ctx);
  aliasBy(ctx, ctx.chains.all(), 'name');

  ctx.deployments = await buildDeployments(scenInfo, ctx);
  ctx.chains.attachDeployments(ctx.deployments);
  aliasBy(ctx, ctx.deployments.all(), 'name');
  aliasBy(ctx, ctx.deployments.starports(), 'ctxKey');
  aliasBy(ctx, ctx.deployments.cashTokens(), 'ctxKey');

  // xxx todo:wn for now including a convenient alias for eth deployment of starport and cashToken due to pervasive use
  ctx.starport = ctx.ethStarport;
  ctx.cashToken = ctx.ethCashToken;

  ctx.actors = await buildActors(scenInfo.actors, scenInfo.default_actor, ctx);
  ctx.tokens = await buildTokens(scenInfo.tokens, scenInfo, ctx);
  ctx.chainSpec = await buildChainSpec(scenInfo.chain_spec, scenInfo.validators, scenInfo.tokens, ctx);
  ctx.prices = await buildPrices(scenInfo.prices, scenInfo.tokens, ctx);
  ctx.validators = await buildValidators(scenInfo.validators, ctx);
  ctx.trxReq = await buildTrxReq(ctx);
  ctx.chain = await buildChain(ctx);
  ctx.eventTracker = await buildEventTracker(ctx);
  ctx.sleep = ctx.__sleep.bind(ctx);
  ctx.keyring = ctx.actors.keyring;
  ctx.api = ctx.tryApi();

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
