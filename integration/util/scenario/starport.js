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

  async executeProposal(title, extrinsics, awaitEvent = true, awaitNotice = false) {
    let encodedCalls = extrinsics.map(encodeCall);
    let result = await this.starport.methods.executeProposal(title, encodedCalls).send({ from: this.ctx.eth.root() });
    let event;
    let notice;
    if (awaitNotice) {
      notice = await this.ctx.chain.waitForNotice();
    }
    if (awaitEvent) {
      event = await this.ctx.chain.waitForEthProcessEvent('cash', 'ExecutedGovernance');
    }
    return {
      event,
      notice,
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

  async upgrade(impl, upgradeCall=null) {
    if (upgradeCall) {
      await this.proxyAdmin.methods.upgradeAndCall(
        this.starport._address,
        impl._address,
        upgradeCall
      ).send({ from: this.ctx.eth.root() });
    } else {
      await this.proxyAdmin.methods.upgrade(this.starport._address, impl._address).send({ from: this.ctx.eth.root() });
    }

    this.starport = this.ctx.eth.__getContractAtAbi(impl._jsonInterface, this.proxy._address);
  }

  async supplyCap(tokenLookup) {
    let token = this.ctx.tokens.get(tokenLookup);

    return this.starport.methods.supplyCaps(token.ethAddress()).call();
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
  let proxyAdmin = await ctx.eth.__deploy('ProxyAdmin', [], { from: ctx.eth.root() });
  let starportImpl = await ctx.eth.__deploy('Starport', [ctx.cashToken.ethAddress(), ctx.eth.root()]);
  let proxy = await ctx.eth.__deploy('TransparentUpgradeableProxy', [
    starportImpl._address,
    proxyAdmin._address,
    "0x"
  ], { from: ctx.eth.root() });
  let starport = await ctx.eth.__getContractAt('Starport', proxy._address);
  if (validators.length > 0) {
    await starport.methods.changeAuthorities(validators).send({ from: ctx.eth.root(), gas: 4_000_00 });
  }

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
