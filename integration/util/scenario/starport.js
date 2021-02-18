const { getValidatorsInfo } = require('./validator');
const { EtherToken } = require('./token');
const { encodeCall } = require('../substrate');

class Starport {
  constructor(starport, proxyAdmin, starportImpl, proxy, starportTopics, ctx) {
    this.starport = starport;
    this.proxyAdmin = proxyAdmin;
    this.starportImpl = starportImpl;
    this.proxy = proxy;
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

  async setSupplyCap(token, amount) {
    let weiAmount = token.toWeiAmount(amount);

    return await this.starport.methods.setSupplyCap(token.ethAddress(), weiAmount).send({ from: this.ctx.eth.root() });
  }

  async executeProposal(title, extrinsics, awaitEvent = true) {
    let encodedCalls = extrinsics.map(encodeCall);
    let result = await this.starport.methods.executeProposal(title, encodedCalls).send({ from: this.ctx.eth.root() });
    let event;
    if (awaitEvent) {
      event = await this.ctx.chain.waitForEthProcessEvent('cash', 'ExecutedGovernance');
    }
    return {
      event,
      result
    };
  }

  async isNoticeUsed(noticeHash) {
    return await this.starport.methods.isNoticeUsed(noticeHash).call();
  }

  async execTrxRequest(actorLookup, trxReq, awaitEvent = true) {
    let actor = this.ctx.actors.get(actorLookup);

    let event;
    let tx = this.starport.methods.execTrxRequest(trxReq).send({ from: actor.ethAddress() });
    if (awaitEvent) {
      // TODO: Pass in log id?
      event = await this.ctx.chain.waitForChainProcessed();
    }
    return {
      tx,
      event
    };
  }

  async invoke(notice, signaturePairs) {
    let encoded = notice.EncodedNotice;
    let signatures = signaturePairs.map(([signer, sig]) => sig.toHex());
    return await this.starport.methods.invoke(encoded, signatures).send({ from: this.ctx.eth.defaultFrom, gas: 5000000 });
  }

  async invokeChain(target, notices) {
    let encodedTarget = target.EncodedNotice;
    let encodedNotices = notices.map((n) => typeof(n) === 'string' ? n : n.EncodedNotice);
    return await this.starport.methods.invokeChain(encodedTarget, encodedNotices).send({ from: this.ctx.eth.defaultFrom, gas: 5000000 });
  }
}

async function buildStarport(starportInfo, validatorsInfoHash, ctx) {
  ctx.log("Deploying Starport...");
  if (!ctx.cashToken) {
    throw new Error(`Cannot deploy Starport without first deploying Cash Token`);
  }
  let validatorsInfo = await getValidatorsInfo(validatorsInfoHash, ctx);
  let validators = validatorsInfo.map(([_, v]) => v.eth_account);

  // Deploy Proxies and Starport
  let proxyAdmin = await ctx.eth.__deployContract(ctx.__getContractsFile(), 'ProxyAdmin', [], { from: ctx.eth.root() });
  let starportImpl = await ctx.eth.__deployContract(ctx.__getContractsFile(), 'Starport', [ctx.cashToken.ethAddress(), ctx.eth.root()]);
  let proxy = await ctx.eth.__deployContract(ctx.__getContractsFile(), 'TransparentUpgradeableProxy', [
    starportImpl._address,
    proxyAdmin._address,
    "0x"
  ], { from: ctx.eth.root() });
  let starport = await ctx.eth.__getContractAt(ctx.__getContractsFile(), 'Starport', proxy._address);
  await starport.methods.changeAuthorities(validators).send({ from: ctx.eth.root() });

  let starportTopics = Object.fromEntries(starport
    ._jsonInterface
    .filter(e => e.type === 'event')
    .map(e => [e.name, e.signature]));

  return new Starport(starport, proxyAdmin, starportImpl, proxy, starportTopics, ctx);
}

module.exports = {
  buildStarport,
  Starport
};
