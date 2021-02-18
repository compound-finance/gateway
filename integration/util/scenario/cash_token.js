const { readContractsFile } = require('../ethereum');
const { Token } = require('./token');

class CashToken extends Token {
  constructor(cashToken, owner, ctx) {
    super('cash', 'CASH', 'Cash Token', 6, cashToken, owner, ctx);

    this.cashToken = cashToken;
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

  async cashIndex() {
    return this.cashToken.methods.getCashIndex().call();
  }

  async getCashPrincipal(actorLookup) {
    let actor = this.ctx.actors.get(actorLookup);
    return Number(await this.token.methods.cashPrincipal(actor.ethAddress()).call());
  }

  async getTotalCashPrincipal() {
    return Number(await this.token.methods.totalCashPrincipal().call());
  }

  async getCashYieldAndIndex() {
    // TODO: How to parse result?
    return await this.token.methods.cashYieldAndIndex().call();
  }
}

async function buildCashToken(cashTokenInfo, ctx, owner) {
  ctx.log("Deploying cash token...");
  let initial_yield_index = cashTokenInfo.initial_yield_index;

  let cashToken = await ctx.eth.__deploy('CashToken', [owner, ctx.__initialYield(), initial_yield_index, ctx.__initialYieldStart()]);

  return new CashToken(cashToken, owner, ctx);
}

module.exports = {
  CashToken,
  buildCashToken
};
