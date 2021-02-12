const { readContractsFile } = require('../ethereum');
const { Token } = require('./token');

class CashToken {
  constructor(cashToken, owner, ctx) {
    this.cashToken = cashToken;
    this.owner = owner;
    this.ctx = ctx;
  }

  ethAddress() {
    return this.cashToken._address;
  }

  toToken() {
    return new Token(
      'cash', // TODO: Consider pulling these from the token itself
      'CASH',
      'Cash Token',
      6,
      this.cashToken,
      this.owner,
      this.ctx
    );
  }
}

async function buildCashToken(cashTokenInfo, ctx, owner) {
  ctx.log("Deploying cash token...");
  let cashToken = await ctx.eth.__deployContract(ctx.__getContractsFile(), 'CashToken', [owner])

  return new CashToken(cashToken, owner, ctx);
}

module.exports = {
  CashToken,
  buildCashToken
};
