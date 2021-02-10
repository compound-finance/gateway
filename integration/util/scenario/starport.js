const { getValidatorsInfo } = require('./validator');
const { EtherToken } = require('./token');

class Starport {
  constructor(starport, starportTopics, ctx) {
    this.starport = starport;
    this.starportTopics = starportTopics;
    this.ctx = ctx;
  }

  ethAddress() {
    return this.starport._address;
  }

  topics() {
    return this.starportTopics;
  }

  async approve(actorLookup, amount, collateral) {
    let actor = this.ctx.actors.get(actorLookup);
    let token = this.ctx.tokens.get(collateral);

    await token.approve(actor, this.ethAddress(), amount);
  }

  async lockEth(actorLookup, weiAmount) {
    let actor = this.ctx.actors.get(actorLookup);
    // Note: we use gas price = 0 for tests to prevent this from scewing the eth balance of the user
    return await this.starport.methods.lockEth().send({ value: weiAmount, from: actor.ethAddress(), gasPrice: "0" });
  }

  async lock(actorLookup, amount, collateral, approve=true) {
    let actor = this.ctx.actors.get(actorLookup);
    let token = this.ctx.tokens.get(collateral);
    let weiAmount = token.toWeiAmount(amount);

    if (token instanceof EtherToken) {
      return await this.lockEth(actor, weiAmount);
    } else {
      if (approve) {
        await this.approve(actor, amount, collateral);
      }
      return await this.starport.methods.lock(weiAmount, token.ethAddress()).send({ from: actor.ethAddress() });
    }
  }

  async invoke(notice, signaturePairs) {
    let encoded = notice.EncodedNotice;
    let signatures = signaturePairs.map(([signer, sig]) => sig.toHex());
    this.ctx.log({ encoded, signatures });
    return await this.starport.methods.invoke(encoded, signatures).send({ from: this.ctx.eth.defaultFrom });
  }
}

async function buildStarport(starportInfo, validatorsInfoHash, ctx) {
  ctx.log("Deploying Starport...");
  if (!ctx.cashToken) {
    throw new Error(`Cannot deploy Starport without first deploying Cash Token`);
  }
  let validatorsInfo = await getValidatorsInfo(validatorsInfoHash, ctx);
  let validators = validatorsInfo.map(([_, v]) => v.eth_account);

  let starport = await ctx.eth.__deployContract(ctx.__getContractsFile(), 'Starport', [ctx.cashToken.ethAddress(), validators]);

  let starportTopics = Object.fromEntries(starport
    ._jsonInterface
    .filter(e => e.type === 'event')
    .map(e => [e.name, e.signature]));

  return new Starport(starport, starportTopics, ctx);
}

module.exports = {
  buildStarport,
  Starport
};
