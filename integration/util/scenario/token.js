const { instantiateInfo } = require('./scen_info');
const BigNumber = require('bignumber.js');
const { lookupBy } = require('../util');
const { descale } = require('../substrate');

class Token {
  constructor(ticker, symbol, name, decimals, priceTicker, liquidityFactor, token, owner, chainName, ctx) {
    this.ticker = ticker;
    this.symbol = symbol;
    this.name = name;
    this.decimals = decimals;
    this.priceTicker = priceTicker;
    this.liquidityFactor = liquidityFactor;
    this.token = token;
    this.owner = owner;
    this.chainName = chainName;
    this.ctx = ctx;
  }

  chain() {
    return this.ctx.chains.find(this.chainName);
  }

  ethAddress() {
    return this.token._address;
  }

  toTrxArg() {
    return `${this.chain().nameAsPascalCase()}:${this.ethAddress()}`;
  }

  toChainAsset(lower = false) {
    const returnValue = {};
    returnValue[this.chain().nameAsPascalCase()] = lower ? this.ethAddress().toLowerCase() : this.ethAddress();
    return returnValue;
  }

  toTokenObject() {
    return {
      name: this.name,
      symbol: this.symbol,
      decimals: this.decimals,
      address: this.ethAddress()
    };
  }

  toWeiAmount(tokenAmount) {
    return dec(tokenAmount, this.decimals);
  }

  toTokenAmount(weiAmount) {
    return undec(weiAmount, this.decimals);
  }

  show() {
    return this.symbol;
  }

  showAmount(weiAmount) {
    return `${this.toTokenAmount(weiAmount)} ${this.symbol}`;
  }

  lockEventName() {
    return 'Locked';
  }

  async getBalance(actorLookup) {
    let balanceWei;
    if (typeof(actorLookup) === 'string' && actorLookup.slice(0, 2) === '0x') {
      balanceWei = await this.token.methods.balanceOf(actorLookup).call();
    } else {
      let actor = this.ctx.actors.get(actorLookup);
      balanceWei = await this.token.methods.balanceOf(actor.ethAddress()).call();
    }

    return this.toTokenAmount(balanceWei);
  }

  async setSupplyCap(tokenAmount) {
    const chain = this.chain();
    if (!chain || !chain.starport) {
      throw new Error(`Ctx: starport must be set before using set supply cap`);
    }
    chain.starport.setSupplyCap(this, tokenAmount);
  }

  async transfer(fromLookup, toLookup, tokenAmount) {
    let fromActor = this.ctx.actors.get(fromLookup);
    let toActor = this.ctx.actors.get(toLookup);
    let weiAmount = this.toWeiAmount(tokenAmount);

    await this.token.methods.transfer(toActor.ethAddress(), weiAmount).send({from: fromActor.ethAddress()});
  }

  async setBalance(actorLookup, tokenAmount) {
    let weiAmount = this.toWeiAmount(tokenAmount);
    if (!this.ctx.actors) {
      throw new Error(`Ctx: actors must be set before using set balance`);
    }
    let actor = this.ctx.actors.get(actorLookup);
    this.ctx.log(`Setting balance for ${actor.name} to ${weiAmount} ${this.symbol}...`);
    let currentBalance = await this.getBalance(actor);
    if (currentBalance > weiAmount) {
      throw new Error(`setBalance failed, unwilling to reduce balance for ${actor.name}: currentBalance=${currentBalance}, weiAmount=${weiAmount}`)
    }

    await this.token.methods.transfer(actor.ethAddress(), weiAmount.minus(currentBalance)).send({from: this.owner});

    // Double check the balance is properly set now
    let newBalance = await this.getBalance(actor);
    if (newBalance.toString() !== tokenAmount.toString()) { // Use string comparisons since these numbers are weird
      console.warn(`setBalance failed, balance not set for ${actor.name}: newBalance=${newBalance}, weiAmount=${weiAmount}`)
    }
  }

  async approve(actorLookup, spender, tokenAmount, force=false) {
    let weiAmount = this.toWeiAmount(tokenAmount);
    let actor = this.ctx.actors.get(actorLookup);

    let approval = await this.token.methods.allowance(actor.ethAddress(), spender).call();
    if (force || approval < weiAmount) {
      await this.token.methods.approve(spender, weiAmount).send({from: actor.ethAddress()});
    }
  }

  async getAssetInfo(field = undefined) {
    let assetRes = await this.ctx.getApi().query.cash.supportedAssets(this.toChainAsset());
    if(!assetRes.isSome) {
      throw new Error(`SupportedAssets field not found ${this.toChainAsset()}`)
    }

    let unwrapped = assetRes.unwrap();
    if (field) {
      if (unwrapped.hasOwnProperty(field)) {
        return unwrapped[field];
      } else {
        throw new Error(`No such field ${field} on ${JSON.stringify(unwrapped)}`);
      }
    } else {
      return unwrapped;
    }
  }

  async getPrice() {
    if (['USD', 'CASH'].includes(this.priceTicker)) {
      return 1.0;
    } else {
      const ticker = await this.getAssetInfo('ticker');
      let price = await this.ctx.getApi().query.oracle.prices(ticker);
      if (price.isSome) {
        return descale(price.unwrap(), 6);
      } else {
        return null;
      }
    }
  }

  async getLiquidityFactor() {
    let liquidityFactor = await this.getAssetInfo('liquidity_factor');
    return descale(liquidityFactor, 18);
  }

  async totalChainSupply() {
    return this.toTokenAmount(await this.ctx.getApi().query.cash.totalSupplyAssets(this.toChainAsset()));
  }

  async totalChainBorrows() {
    return this.toTokenAmount(await this.ctx.getApi().query.cash.totalBorrowAssets(this.toChainAsset()));
  }
}

class EtherToken extends Token {
  constructor(liquidityFactor, ctx) {
    super('ether', 'ETH', 'Ether', 18, 'ETH', liquidityFactor, null, null, 'eth', ctx);
  }

  ethAddress() {
    return '0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE';
  }

  async getBalance(actorLookup) {
    let balanceWei = await this.ctx.eth.ethBalance(actorLookup);

    return this.toTokenAmount(balanceWei);
  }

  async setBalance(actorLookup, tokenAmount) {
    throw new Error(`Cannot set balance for ether token`);
  }

  async approve(actorLookup, spender, tokenAmount, force=false) {
    this.ctx.log("Not setting approval for ether token");
  }
}

class Tokens {
  constructor(tokens, ctx) {
    this.tokens = tokens;
    this.ctx = ctx;
  }

  all() {
    return this.tokens;
  }

  get(lookup) {
    return lookupBy(Token, 'ticker', this.tokens, lookup);
  }

  tokenObjects() {
    return Object.fromEntries(this.tokens.map((token) =>
      [token.symbol, token.toTokenObject()]));
  }

  filterByChain(chainName) {
    return new Tokens(this.tokens.filter((e) => e.chainName === chainName), this.ctx);
  }
}

function tokenInfoMap(ctx) {
  let accounts = ctx.eth.accounts;

  return {
    zrx: {
      chain: 'eth',
      build: 'zrx.json',
      contract: 'ZRXToken',
      decimals: 18,
      constructor_args: [],
      supply_cap: 1000000,
      liquidity_factor: 0.5,
    },
    maticZrx: {
      chain: 'matic',
      build: 'zrx.json',
      contract: 'ZRXToken',
      decimals: 18,
      constructor_args: [],
      supply_cap: 1000000,
      liquidity_factor: 0.5,
      price_ticker: "ZRX",
    },
    dai: {
      chain: 'eth',
      build: 'dai.json',
      contract: 'Dai',
      decimals: 18,
      constructor_args: [0], // TODO: ChainId
      supply_cap: 1000000,
      liquidity_factor: 0.8,
    },
    comp: {
      chain: 'eth',
      build: 'compound.json',
      contract: 'Comp',
      decimals: 18,
      constructor_args: [accounts[0]],
      supply_cap: 1000000,
      liquidity_factor: 0.75,
    },
    bat: {
      chain: 'eth',
      build: 'bat.json',
      contract: 'BAToken',
      decimals: 18,
      constructor_args: ['0x0000000000000000000000000000000000000000', accounts[0], 0, 0],
      supply_cap: 1000000,
      liquidity_factor: 0.3,
    },
    wbtc: {
      chain: 'eth',
      build: 'wbtc.json',
      contract: 'WBTC',
      decimals: 8,
      constructor_args: [],
      supply_cap: 1000000,
      liquidity_factor: 0.6,
      price_ticker: 'BTC',
    },
    usdc: {
      chain: 'eth',
      build: 'FiatTokenV1.json',
      contract: 'FiatTokenV1',
      decimals: 6,
      constructor_args: [],
      supply_cap: 1000000,
      liquidity_factor: 0.8,
      price_ticker: 'USD',
      afterDeploy: async (contract, owner) => {
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
        await contract.methods.configureMinter(owner, dec(1000000, 6)).send({ from: owner });
        await contract.methods.mint(owner, dec(1000000, 6)).send({ from: owner });
      }
    },
    fee: {
      chain: 'eth',
      build: 'contracts.json',
      contract: 'FeeToken',
      decimals: 6,
      constructor_args: [1000000e6, "Fee Token", 6, "FEE"],
      supply_cap: 1000000,
      liquidity_factor: 0.8,
      price_ticker: 'USD'
    }
  }
}

function dec(weiAmount, decimals) {
  return new BigNumber(`${weiAmount.toString()}e${decimals}`);
}

function undec(tokenAmount, decimals) {
  return Number(`${tokenAmount.toString()}e-${decimals}`);
}

async function buildToken(ticker, tokenInfo, ctx) {
  ctx.log(`Deploying ${ticker} to ${tokenInfo.chain}...`);
  const chain = ctx.chains.find(tokenInfo.chain);
  if(!chain) {
    throw new Error(`while initializing ${ticker} chain not found ${tokenInfo.chain}`)
  }

  let owner = chain.defaultFrom;
  let tokenContract;
  if (tokenInfo.address) {
    tokenContract = await chain.__getContractAtFull(ctx.__getContractsFile(tokenInfo.build), tokenInfo.contract, tokenInfo.address, { from: owner });
  } else {
    tokenContract = await chain.__deployFull(ctx.__getContractsFile(tokenInfo.build), tokenInfo.contract, tokenInfo.constructor_args, { from: owner });
  }
  if (typeof (tokenInfo.afterDeploy) === 'function') {
    await tokenInfo.afterDeploy(tokenContract, owner);
  }
  let symbol = await tokenContract.methods.symbol().call();
  let name = await tokenContract.methods.name().call();
  let decimals = Number(await tokenContract.methods.decimals().call());
  let priceTicker = tokenInfo.price_ticker || symbol;
  let liquidityFactor = tokenInfo.liquidity_factor;
  let token = new Token(ticker, symbol, name, decimals, priceTicker, liquidityFactor, tokenContract, owner, tokenInfo.chain, ctx);

  if (tokenInfo.balances) {
    await Object.entries(tokenInfo.balances).reduce(async (acc, [actor, amount]) => {
      await acc;
      await token.setBalance(actor, amount);
    }, Promise.resolve(undefined));
  }

  if (tokenInfo.supply_cap) {
    await token.setSupplyCap(tokenInfo.supply_cap);
  }

  return token;
}

async function getTokensInfo(tokensInfoHash, ctx) {
  return await instantiateInfo(tokensInfoHash, 'Token', 'token', tokenInfoMap(ctx));
}

async function buildTokens(tokensInfoHash, scenInfo, ctx) {
  ctx.log("Deploying Erc20 Tokens...");

  let tokensInfo = await getTokensInfo(tokensInfoHash, ctx);
  let tokens = await tokensInfo.reduce(async (acc, [ticker, tokenInfo]) => {
    return [
      ...await acc,
      await buildToken(ticker, tokenInfo, ctx)
    ];
  }, Promise.resolve([]));

  let etherToken = new EtherToken(scenInfo.eth_liquidity_factor, ctx);
  tokens.push(etherToken);
  if (scenInfo.eth_supply_cap) {
    await etherToken.setSupplyCap(scenInfo.eth_supply_cap);
  }
  tokens.push(ctx.ethCashToken);

  // alias tokens.eth, tokens.matic, ...
  const returnValue = new Tokens(tokens, ctx);
  ctx.chains.all().forEach((chain) => {
    returnValue[chain.name] = returnValue.filterByChain(chain.name);
  })

  return returnValue;
}

module.exports = {
  buildToken,
  buildTokens,
  getTokensInfo,
  EtherToken,
  Token,
  Tokens,
};
