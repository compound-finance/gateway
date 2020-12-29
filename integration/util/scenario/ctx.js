const { debug, log, error } = require('../log');
const { merge, sleep } = require('../util');
const path = require('path');

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
const { buildPrices } = require('./price');

class Ctx {
  constructor(scenInfo) {
    this.scenInfo = scenInfo;
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
    return process.env['CHAIN_BIN'] || this.scenInfo['target'] || path.join(__dirname, '..', '..', '..', 'target', this.__profile(), 'compound-chain')
  }

  __typesFile() {
    return process.env['TYPES_FILE'] || this.scenInfo['types_file'] || path.join(__dirname, '..', '..', '..', 'types.json')
  }

  __provider() {
    return process.env['PROVIDER'] || this.scenInfo['eth_opts']['provider'];
  }

  debug(...msg) {
    debug(...msg);
  }

  log(...msg) {
    log(...msg);
  }

  error(...msg) {
    error(...msg);
  }

  api() {
    return this.validators.api();
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
    if (this.validators) {
      await this.validators.teardown();
    }

    if (this.eth) {
      await this.eth.teardown();
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
  debug(() => `Builing ctx with scenInfo=${JSON.stringify(scenInfo, null, 2)}`);
  debug(() => `test=${JSON.stringify(scenInfo.chain_spec, null, 2)}`);
  let ctx = new Ctx(scenInfo);
  ctx.eth = await buildEth(scenInfo.eth_opts, ctx);
  ctx.cashToken = await buildCashToken(scenInfo.cash_token, ctx);
  ctx.starport = await buildStarport(scenInfo.starport, scenInfo.validators, ctx);
  ctx.actors = await buildActors(scenInfo.actors, scenInfo.default_actor, ctx);
  ctx.tokens = await buildTokens(scenInfo.tokens, ctx);
  ctx.chainSpec = await buildChainSpec(scenInfo.chain_spec, scenInfo.validators, scenInfo.tokens, ctx);
  ctx.validators = await buildValidators(scenInfo.validators, ctx);
  ctx.trxReq = await buildTrxReq(ctx);
  ctx.chain = await buildChain(ctx);
  ctx.prices = await buildPrices(scenInfo.tokens, ctx);

  aliasBy(ctx, ctx.actors.all(), 'name');
  aliasBy(ctx, ctx.tokens.all(), 'ticker');
  aliasBy(ctx, ctx.validators.all(), 'name');

  return ctx;
}

module.exports = {
  Ctx,
  buildCtx,
};
