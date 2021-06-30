const { readContractsFile } = require('../ethereum');
const { Token } = require('./token');

class CashToken extends Token {
  constructor(cashToken, proxyAdmin, cashImpl, proxy, liquidityFactor, owner, chainName, ctx) {
    super('cash', 'CASH', 'Cash Token', 6, 'CASH', liquidityFactor, cashToken, owner, chainName, ctx);

    this.cashToken = cashToken;
    this.proxyAdmin = proxyAdmin;
    this.cashImpl = cashImpl;
    this.proxy = proxy;
    this.chainName = chainName;
    this.ctxKey = `${chainName}CashToken`;
  }

  toTrxArg() {
    return `CASH`;
  }

  toWeiAmount(tokenAmount) {
    if (tokenAmount === 'Max' || tokenAmount === 'MAX') {
      return tokenAmount;
    } else {
      return super.toWeiAmount(tokenAmount)
    }
  }

  lockEventName() {
    return 'LockedCash';
  }

  async cashIndex() {
    return this.cashToken.methods.getCashIndex().call();
  }

  async getCashPrincipal(actorLookup) {
    let actor = this.ctx.actors.get(actorLookup);
    return Number(await this.cashToken.methods.cashPrincipal(actor.ethAddress()).call());
  }

  async getTotalCashPrincipal() {
    return Number(await this.cashToken.methods.totalCashPrincipal().call());
  }

  async cashYieldStart() {
    return await this.cashToken.methods.cashYieldStart().call();
  }

  async getCashYieldAndIndex() {
    let { yield: theYield, index } = await this.cashToken.methods.cashYieldAndIndex().call();
    return { yield: theYield, index };
  }

  async nextCashYieldStart() {
    return await this.cashToken.methods.nextCashYieldStart().call();
  }

  async getNextCashYieldAndIndex() {
    let { yield: theYield, index } = await this.cashToken.methods.nextCashYieldAndIndex().call();
    return { yield: theYield, index };
  }

  async upgradeTo(version) {
    let newImpl = await this.ctx.eth.__deploy('CashToken', [this.ctx.starport.ethAddress()], { version });
    await this.upgrade(newImpl);
  }

  async getName() {
    return await this.cashToken.methods.name().call();
  }

  async getSymbol() {
    return await this.cashToken.methods.symbol().call();
  }

  async getLiquidityFactor() {
    return 1;
  }


  async upgrade(impl, upgradeCall = null) {
    if (upgradeCall) {
      await this.proxyAdmin.methods.upgradeAndCall(
        this.cashToken._address,
        impl._address,
        upgradeCall
      ).send({ from: this.ctx.eth.root() });
    } else {
      let tx = await this.proxyAdmin.methods.upgrade(this.cashToken._address, impl._address).send({ from: this.ctx.eth.root() });
    }

    this.cashToken = this.ctx.eth.__getContractAtAbi(impl._jsonInterface, this.proxy._address);
  }
}

async function buildCashToken(cashTokenInfo, ctx, owner, chain) {
  ctx.log(`Deploying cash token to ${chain.name}...`);

  if (cashTokenInfo.existing) {
    // Use existing Starport information
    ['proxy_admin', 'proxy', 'cash_token', 'cash_impl'].forEach((key) => {
      if (!cashTokenInfo.existing[key]) {
        throw new Error(`Existing Cash Token missing property: ${key}`);
      }
    });

    let proxyAdmin = cashTokenInfo.existing.proxy_admin;
    let proxy = await chain.__getContractAt('TransparentUpgradeableProxy', cashTokenInfo.existing.proxy);
    let cashToken = await chain.__getContractAt('CashToken', cashTokenInfo.existing.cash_token);
    let cashImpl = await chain.__getContractAt('CashToken', cashTokenInfo.existing.cash_impl);

    // TODO: Owner?
    return new CashToken(cashToken, proxyAdmin, cashImpl, proxy, cashTokenInfo.liquidity_factor, owner, chain.name, ctx);
  } else {
    let proxyAdmin = await chain.__deploy('ProxyAdmin', [], { from: chain.root() });
    let cashImpl = await chain.__deploy('CashToken', [owner]);
    let proxy = await chain.__deploy('TransparentUpgradeableProxy', [
      cashImpl._address,
      proxyAdmin._address,
      cashImpl.methods.initialize(ctx.__initialYield(), ctx.__initialYieldStart()).encodeABI()
    ], { from: chain.root() });
    let cashToken = await chain.__getContractAt('CashToken', proxy._address);

    return new CashToken(cashToken, proxyAdmin, cashImpl, proxy, cashTokenInfo.liquidity_factor, owner, chain.name, ctx);
  }
}


module.exports = {
  CashToken,
  buildCashToken
};
