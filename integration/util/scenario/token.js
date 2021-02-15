const { instantiateInfo } = require('./scen_info');
const BigNumber = require('bignumber.js');
const { lookupBy } = require('../util');

class Token {
  constructor(ticker, symbol, name, decimals, token, owner, ctx) {
    this.ticker = ticker;
    this.symbol = symbol;
    this.name = name;
    this.decimals = decimals;
    this.token = token;
    this.owner = owner;
    this.ctx = ctx;
  }

  ethAddress() {
    return this.token._address;
  }

  toTrxArg() {
    return `Eth:${this.ethAddress()}`;
  }

  toChainAsset(lower = false) {
    return { Eth: lower ? this.ethAddress().toLowerCase() : this.ethAddress() };
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

  async getBalance(actorLookup) {
    let actor = this.ctx.actors.get(actorLookup);
    let balanceWei = await this.token.methods.balanceOf(actor.ethAddress()).call();

    return this.toTokenAmount(balanceWei);
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
      throw new Error(`setBalance failed, balance not set for ${actor.name}: newBalance=${newBalance}, weiAmount=${weiAmount}`)
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

  async getSymbol() {
    return (await this.ctx.api().query.cash.assetSymbols(this.toChainAsset())).unwrap();
  }

  async getPrice() {
    return Number(await this.ctx.api().query.cash.prices(await this.getSymbol()));
  }
}

class EtherToken extends Token {
  constructor(ctx) {
    super('ether', 'ETH', 'Ether', 18, null, null, ctx);
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
}

function tokenInfoMap(ctx) {
  let accounts = ctx.eth.accounts;

  return {
    zrx: {
      build: 'zrx.json',
      contract: 'ZRXToken',
      decimals: 18,
      constructorArgs: [],
    },
    dai: {
      build: 'dai.json',
      contract: 'Dai',
      decimals: 18,
      constructorArgs: [0] // TODO: ChainId
    },
    comp: {
      build: 'compound.json',
      contract: 'Comp',
      decimals: 18,
      constructorArgs: [accounts[0]]
    },
    bat: {
      build: 'bat.json',
      contract: 'BAToken',
      decimals: 18,
      constructorArgs: ['0x0000000000000000000000000000000000000000', accounts[0], 0, 0]
    },
    wbtc: {
      build: 'wbtc.json',
      contract: 'WBTC',
      decimals: 8,
      constructorArgs: []
    },
    usdc: {
      build: 'FiatTokenV1.json',
      contract: 'FiatTokenV1',
      decimals: 6,
      constructorArgs: [],
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
  ctx.log(`Deploying ${ticker}...`);

  let owner = ctx.eth.defaultFrom;
  let tokenContract = await ctx.eth.__deployContract(ctx.__getContractsFile(tokenInfo.build), tokenInfo.contract, tokenInfo.constructorArgs, {from: owner});
  if (typeof (tokenInfo.afterDeploy) === 'function') {
    await tokenInfo.afterDeploy(tokenContract, owner);
  }
  let symbol = await tokenContract.methods.symbol().call();
  let name = await tokenContract.methods.name().call();
  let decimals = Number(await tokenContract.methods.decimals().call());
  let token = new Token(ticker, symbol, name, decimals, tokenContract, owner, ctx);

  if (tokenInfo.balances) {
    await Object.entries(tokenInfo.balances).reduce(async (acc, [actor, amount]) => {
      await acc;
      await token.setBalance(actor, amount);
    }, Promise.resolve(undefined));
  }

  return token;
}

async function getTokensInfo(tokensInfoHash, ctx) {
  return await instantiateInfo(tokensInfoHash, 'Token', 'token', tokenInfoMap(ctx));
};

async function buildTokens(tokensInfoHash, ctx) {
  ctx.log("Deploying Erc20 Tokens...");

  let tokensInfo = await getTokensInfo(tokensInfoHash, ctx);
  let tokens = await tokensInfo.reduce(async (acc, [ticker, tokenInfo]) => {
    return [
      ...await acc,
      await buildToken(ticker, tokenInfo, ctx)
    ];
  }, Promise.resolve([]));

  tokens.push(new EtherToken(ctx));
  tokens.push(ctx.cashToken);

  return new Tokens(tokens, ctx);
}

module.exports = {
  buildToken,
  buildTokens,
  getTokensInfo,
  EtherToken,
  Token,
  Tokens,
};
